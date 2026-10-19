use nerva_qemu::{
    capture::Frame,
    keyboard::{Key, Keyboard},
    motor_decoder::MotorDecoder,
    qmp::Qmp,
    signals::{ChannelRate, MotorPulse},
    vision_encoder::{Encoding, VisionEncoder, grayscale},
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    thread,
    time::Duration,
};

fn read_request(stream: &mut BufReader<UnixStream>) -> Value {
    let mut line = String::new();
    assert!(stream.read_line(&mut line).unwrap() > 0);
    serde_json::from_str(&line).unwrap()
}

fn reply(stream: &mut BufReader<UnixStream>, request: &Value, result: Value) {
    writeln!(
        stream.get_mut(),
        "{}",
        json!({"id": request["id"], "return": result})
    )
    .unwrap();
}

fn fake_qmp<T: Send + 'static>(
    handler: impl FnOnce(BufReader<UnixStream>) -> T + Send + 'static,
) -> (Qmp, thread::JoinHandle<T>) {
    let (client, server) = UnixStream::pair().unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let handle = thread::spawn(move || {
        let mut server = BufReader::new(server);
        server.get_mut().write_all(b"{\"QMP\":").unwrap();
        server
            .get_mut()
            .write_all(b"{\"version\":{}}}\r\n")
            .unwrap();
        let capabilities = read_request(&mut server);
        assert_eq!(capabilities["execute"], "qmp_capabilities");
        reply(&mut server, &capabilities, json!({}));
        handler(server)
    });
    (Qmp::from_stream(client).unwrap(), handle)
}

#[test]
fn qmp_handles_fragmented_json_events_escaping_and_remote_errors() {
    let (mut qmp, server) = fake_qmp(|mut server| {
        let request = read_request(&mut server);
        assert_eq!(request["arguments"]["filename"], "/tmp/a\"b\\c.ppm");
        server
            .get_mut()
            .write_all(b"{\"event\":\"STOP\"}\r\n")
            .unwrap();
        let response = json!({"id": request["id"], "return": {"accepted": true}}).to_string();
        for part in response.as_bytes().chunks(3) {
            server.get_mut().write_all(part).unwrap();
        }
        server.get_mut().write_all(b"\r\n").unwrap();
        let rejected = read_request(&mut server);
        writeln!(
            server.get_mut(),
            "{}",
            json!({"id": rejected["id"], "error": {"class": "GenericError", "desc": "rejected"}})
        )
        .unwrap();
        let recovery = read_request(&mut server);
        reply(&mut server, &recovery, json!({}));
    });
    assert_eq!(
        qmp.execute("screendump", json!({"filename": "/tmp/a\"b\\c.ppm"}))
            .unwrap()["accepted"],
        true
    );
    assert!(
        qmp.execute("bad-command", json!({}))
            .unwrap_err()
            .to_string()
            .contains("rejected")
    );
    qmp.execute("input-send-event", json!({"events": []}))
        .unwrap();
    server.join().unwrap();
}

#[test]
fn qmp_rejects_wrong_ids_and_does_not_reuse_a_broken_stream() {
    let (mut qmp, server) = fake_qmp(|mut server| {
        read_request(&mut server);
        server
            .get_mut()
            .write_all(b"{\"return\":{},\"id\":999}\n")
            .unwrap();
    });
    assert!(
        qmp.execute("x", json!({}))
            .unwrap_err()
            .to_string()
            .contains("ID mismatch")
    );
    assert!(
        qmp.execute("x", json!({}))
            .unwrap_err()
            .to_string()
            .contains("unusable")
    );
    server.join().unwrap();
}

#[test]
fn qmp_rejects_truncated_responses() {
    let (mut qmp, server) = fake_qmp(|mut server| {
        read_request(&mut server);
        server.get_mut().write_all(b"{\"return\":").unwrap();
    });
    assert!(qmp.execute("x", json!({})).is_err());
    server.join().unwrap();
}

#[test]
fn keyboard_releases_before_switching_deduplicates_and_releases_on_drop() {
    let (qmp, server) = fake_qmp(|mut server| {
        let mut requests = Vec::new();
        loop {
            let mut line = String::new();
            if server.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let request: Value = serde_json::from_str(&line).unwrap();
            reply(&mut server, &request, json!({}));
            requests.push(request);
        }
        requests
    });
    let mut keyboard = Keyboard::new(qmp);
    keyboard.set_keys([Key::A]).unwrap();
    keyboard.set_keys([Key::A]).unwrap();
    keyboard.set_keys([Key::D]).unwrap();
    keyboard.set_keys([]).unwrap();
    keyboard.set_keys([Key::Space]).unwrap();
    drop(keyboard);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 5);
    let switch = &requests[1]["arguments"]["events"];
    assert_eq!(switch[0]["data"]["key"]["data"], "a");
    assert_eq!(switch[0]["data"]["down"], false);
    assert_eq!(switch[1]["data"]["key"]["data"], "d");
    assert_eq!(switch[1]["data"]["down"], true);
    assert_eq!(requests[2]["arguments"]["events"][0]["data"]["down"], false);
    let final_events = requests.last().unwrap()["arguments"]["events"]
        .as_array()
        .unwrap();
    assert_eq!(final_events.len(), Key::COUNT);
    assert!(
        final_events
            .iter()
            .all(|event| event["data"]["down"] == false)
    );
}

#[test]
fn ppm_preserves_whitespace_pixels_and_checks_comments_sizes_and_truncation() {
    let frame = Frame::from_ppm(b"P6\n# comment\n1 1\n255\n\x0a\x20\x23").unwrap();
    assert_eq!(frame.rgb(), [10, 32, 35]);
    assert_eq!(
        Frame::from_ppm(b"P6\r\n1 1\r\n255\r\n\x0a\x20\x23").unwrap(),
        frame
    );
    for bad in [
        b"P6\n1 1\n255\n\x00".as_slice(),
        b"P6\n0 1\n255\n",
        b"P6\n1 1\n65535\n",
        b"P6\n18446744073709551615 2\n255\n",
        b"P3\n1 1\n255\n0 0 0",
    ] {
        assert!(Frame::from_ppm(bad).is_err());
    }
}

#[test]
fn grayscale_averages_visible_area_instead_of_discarding_small_features() {
    let frame = Frame::new(2, 2, vec![255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert!((grayscale(&frame, 1, 1).unwrap()[0] - 0.25).abs() < 1e-6);
    assert!(grayscale(&frame, 3, 3).is_err());
}

#[test]
fn brightness_encoding_emits_rates_without_a_neural_backend() {
    let frame = Frame::new(2, 1, vec![255, 255, 255, 0, 0, 0]).unwrap();
    let mut encoder = VisionEncoder::new(2, 1, 120.0, Encoding::Brightness).unwrap();
    let expected = vec![
        ChannelRate {
            channel: 0,
            rate_hz: 120.0,
        },
        ChannelRate {
            channel: 1,
            rate_hz: 0.0,
        },
    ];
    assert_eq!(encoder.encode(&frame).unwrap(), expected);
    assert_eq!(encoder.encode(&frame).unwrap(), expected);
    assert!(VisionEncoder::new(0, 1, 120.0, Encoding::Brightness).is_err());
    assert!(VisionEncoder::new(1, 1, f32::NAN, Encoding::Brightness).is_err());
}

#[test]
fn change_encoding_uses_separate_on_off_channels_and_a_silent_baseline() {
    let black = Frame::new(1, 1, vec![0; 3]).unwrap();
    let white = Frame::new(1, 1, vec![255; 3]).unwrap();
    for (first, second, active) in [(&black, &white, 0), (&white, &black, 1)] {
        let mut encoder = VisionEncoder::new(1, 1, 1_000.0, Encoding::Change).unwrap();
        assert_eq!(encoder.channel_count(), 2);
        assert!(
            encoder
                .encode(first)
                .unwrap()
                .iter()
                .all(|rate| rate.rate_hz == 0.0)
        );
        let rates = encoder.encode(second).unwrap();
        assert_eq!(
            rates
                .iter()
                .find(|rate| rate.channel == active)
                .unwrap()
                .rate_hz,
            1_000.0
        );
        assert_eq!(
            rates
                .iter()
                .find(|rate| rate.channel != active)
                .unwrap()
                .rate_hz,
            0.0
        );
        assert!(
            encoder
                .encode(second)
                .unwrap()
                .iter()
                .all(|rate| rate.rate_hz == 0.0)
        );
    }
}

fn output(channel: u64, at: u64) -> MotorPulse {
    MotorPulse {
        channel,
        at_us: at,
        amplitude: 1.0,
    }
}

#[test]
fn motor_decoder_abstains_on_silence_ties_and_expired_spikes() {
    let mut decoder = MotorDecoder::new(100, 1).unwrap();
    assert_eq!(decoder.decode(&[], 0).unwrap(), None);
    assert_eq!(
        decoder.decode(&[output(1, 10), output(3, 10)], 20).unwrap(),
        None
    );
    assert_eq!(decoder.decode(&[output(3, 30)], 30).unwrap(), Some(Key::D));
    assert_eq!(decoder.decode(&[], 130).unwrap(), None);
    assert!(decoder.decode(&[], 129).is_err());
}

#[test]
fn invalid_motor_batch_is_rejected_before_mutating_decoder_state() {
    let mut decoder = MotorDecoder::new(100, 1).unwrap();
    assert!(
        decoder
            .decode(&[output(1, 10), output(Key::COUNT as u64, 10)], 10)
            .is_err()
    );
    assert_eq!(decoder.decode(&[], 10).unwrap(), None);
    assert!(decoder.decode(&[output(1, 10)], 20).is_err());
    assert!(decoder.decode(&[output(1, 30)], 20).is_err());
    assert_eq!(decoder.decode(&[output(1, 20)], 20).unwrap(), Some(Key::A));
}

#[test]
fn full_iso_keyboard_has_unique_channels_codes_and_german_positions() {
    use std::collections::BTreeSet;
    assert_eq!(Key::COUNT, 105);
    let codes: BTreeSet<_> = Key::ALL.iter().map(|key| key.qcode()).collect();
    assert_eq!(codes.len(), 105);
    for &key in Key::ALL {
        assert_eq!(Key::from_channel(key.channel()), Some(key));
        assert!(!key.label().is_empty());
    }
    assert_eq!(
        [
            Key::W.channel(),
            Key::A.channel(),
            Key::S.channel(),
            Key::D.channel(),
            Key::Space.channel()
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(Key::from_channel(u64::MAX), None);
    assert_eq!(Key::Y.qcode(), "z");
    assert_eq!(Key::Z.qcode(), "y");
    assert_eq!(Key::UmlautA.qcode(), "apostrophe");
    assert_eq!(Key::UmlautO.qcode(), "semicolon");
    assert_eq!(Key::UmlautU.qcode(), "bracket_left");
    assert_eq!(Key::LessThan.qcode(), "less");
}

#[test]
fn motor_decoder_can_select_every_key_including_navigation_and_numpad() {
    let mut decoder = MotorDecoder::new(100, 1).unwrap();
    for (index, &key) in Key::ALL.iter().enumerate() {
        let at = index as u64 * 101 + 1;
        assert_eq!(
            decoder.decode(&[output(key.channel(), at)], at).unwrap(),
            Some(key)
        );
    }
}

#[test]
fn motor_chords_keep_modifiers_independent_of_regular_key_ties() {
    let mut decoder = MotorDecoder::new(100, 1).unwrap();
    let ctrl = Key::LeftCtrl.channel();
    assert_eq!(
        decoder
            .decode_keys(&[output(ctrl, 10), output(Key::C.channel(), 10)], 20)
            .unwrap(),
        vec![Key::LeftCtrl, Key::C]
    );
    assert_eq!(
        decoder
            .decode_keys(&[output(Key::D.channel(), 30)], 30)
            .unwrap(),
        vec![Key::LeftCtrl]
    );
    assert!(decoder.decode_keys(&[], 130).unwrap().is_empty());
    let mut threshold = MotorDecoder::new(100, 2).unwrap();
    assert!(
        threshold
            .decode_keys(&[output(ctrl, 1), output(Key::C.channel(), 1)], 1)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        threshold
            .decode_keys(&[output(ctrl, 2), output(Key::C.channel(), 2)], 2)
            .unwrap(),
        vec![Key::LeftCtrl, Key::C]
    );
}

#[test]
fn timing_decoder_uses_first_spike_for_full_layout_ties_and_keeps_modifiers() {
    let mut decoder = MotorDecoder::new(100, 1).unwrap().with_timing_ties(true);
    assert_eq!(decoder.decode_keys(&[], 0).unwrap(), Vec::<Key>::new());
    assert_eq!(
        decoder
            .decode_keys(
                &[
                    output(Key::LeftCtrl.channel(), 1),
                    output(Key::F12.channel(), 2),
                    output(Key::UmlautA.channel(), 4),
                ],
                10
            )
            .unwrap(),
        vec![Key::LeftCtrl, Key::F12]
    );
    // A count winner still takes precedence over an earlier first spike.
    assert_eq!(
        decoder
            .decode_keys(&[output(Key::UmlautA.channel(), 12)], 20)
            .unwrap(),
        vec![Key::LeftCtrl, Key::UmlautA]
    );
    assert!(decoder.decode_keys(&[], 120).unwrap().is_empty());
    // Simultaneous first spikes contain no unique timing winner.
    assert_eq!(
        decoder
            .decode(
                &[output(Key::A.channel(), 130), output(Key::D.channel(), 130)],
                140
            )
            .unwrap(),
        None
    );
    assert_eq!(decoder.decode(&[], 240).unwrap(), None);
}

#[test]
fn keyboard_orders_modifier_chords_and_releases_all_105_keys() {
    let (qmp, server) = fake_qmp(|mut server| {
        let mut requests = Vec::new();
        loop {
            let mut line = String::new();
            if server.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let request: Value = serde_json::from_str(&line).unwrap();
            reply(&mut server, &request, json!({}));
            requests.push(request);
        }
        requests
    });
    let mut keyboard = Keyboard::new(qmp);
    keyboard.set_keys([Key::C, Key::LeftCtrl]).unwrap();
    keyboard.set_keys([Key::C, Key::LeftCtrl]).unwrap();
    keyboard.set_keys([Key::A, Key::LeftShift]).unwrap();
    keyboard.set_keys([]).unwrap();
    drop(keyboard);
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    let first = &requests[0]["arguments"]["events"];
    assert_eq!(first[0]["data"]["key"]["data"], "ctrl");
    assert_eq!(first[1]["data"]["key"]["data"], "c");
    let switch = &requests[1]["arguments"]["events"];
    let expected = [("c", false), ("ctrl", false), ("shift", true), ("a", true)];
    for (event, (key, down)) in switch.as_array().unwrap().iter().zip(expected) {
        assert_eq!(event["data"]["key"]["data"], key);
        assert_eq!(event["data"]["down"], down);
    }
    let released = requests[3]["arguments"]["events"].as_array().unwrap();
    assert_eq!(released.len(), 105);
    assert!(released.iter().all(|event| event["data"]["down"] == false));
    let last_modifiers: Vec<_> = released
        .iter()
        .rev()
        .take(8)
        .map(|event| event["data"]["key"]["data"].as_str().unwrap())
        .collect();
    assert!(last_modifiers.contains(&"ctrl") && last_modifiers.contains(&"shift"));
}

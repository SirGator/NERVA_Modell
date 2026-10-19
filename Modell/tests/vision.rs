use nerva::{
    io::{ChannelId, ReceptorSignal},
    primitives::SimTime,
    transduction::{ChannelSpike, PoissonSensoryTransducer, SensoryTransducer},
};
use nerva_qemu::{
    capture::Frame,
    vision_encoder::{Encoding, VisionEncoder},
};
use nerva_vm::Result;

fn encode(
    vision: &mut VisionEncoder,
    poisson: &mut PoissonSensoryTransducer,
    frame: &Frame,
    start: u64,
    end: u64,
) -> Result<Vec<ChannelSpike>> {
    for rate in vision.encode(frame)? {
        poisson.push(ReceptorSignal {
            channel: ChannelId(rate.channel),
            at: SimTime(start),
            value: rate.rate_hz,
        })?;
    }
    let mut spikes = Vec::new();
    poisson.advance_until(SimTime(end), &mut spikes)?;
    Ok(spikes)
}

#[test]
fn adapter_rates_and_nerva_poisson_replay_across_frame_horizons() {
    let frame = Frame::new(2, 1, vec![255; 6]).unwrap();
    let mut whole = VisionEncoder::new(2, 1, 100.0, Encoding::Brightness).unwrap();
    let mut split = VisionEncoder::new(2, 1, 100.0, Encoding::Brightness).unwrap();
    let mut whole_poisson = PoissonSensoryTransducer::new(42);
    let mut split_poisson = PoissonSensoryTransducer::new(42);
    let a = encode(&mut whole, &mut whole_poisson, &frame, 1, 1_000_000).unwrap();
    let mut b = Vec::new();
    for index in 0..10 {
        b.extend(
            encode(
                &mut split,
                &mut split_poisson,
                &frame,
                index * 100_000 + 1,
                (index + 1) * 100_000,
            )
            .unwrap(),
        );
    }
    b.sort_by_key(|spike| (spike.at, spike.channel));
    assert_eq!(a, b);
    assert!(!a.is_empty());
    assert!(encode(&mut split, &mut split_poisson, &frame, 1_000_000, 1_100_000).is_err());
}

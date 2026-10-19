use nerva_experiments::{protocol::*, serial_model::VmModel};
use std::io::{self, Cursor, Read, Write};

struct Wire {
    input: Cursor<Vec<u8>>,
    output: Vec<u8>,
}
impl Read for Wire {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        // Exercise fragmented transport; every read supplies just one byte.
        let length = bytes.len().min(1);
        self.input.read(&mut bytes[..length])
    }
}
impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn fragmented_serial_training_and_prediction_complete_in_order() {
    let mut input = Vec::new();
    for _ in 0..32 {
        input.extend_from_slice(&[REQUEST_MAGIC, TRAIN, 3, 1]);
    }
    input.extend_from_slice(&[REQUEST_MAGIC, PREDICT, 3, 0]);
    input.extend_from_slice(&[REQUEST_MAGIC, FINISH, 0, 0]);
    let mut wire = Wire {
        input: Cursor::new(input),
        output: Vec::new(),
    };
    assert_eq!(serve(&mut wire, &mut VmModel::new().unwrap()).unwrap(), 33);
    assert_eq!(
        &wire.output[128..],
        &[
            RESPONSE_MAGIC,
            0,
            1,
            PREDICT,
            RESPONSE_MAGIC,
            0,
            NO_ACTION,
            FINISH
        ]
    );
}

#[test]
fn invalid_frames_and_disconnects_are_rejected_without_learning() {
    for input in [
        vec![],
        vec![REQUEST_MAGIC, TRAIN, 1],
        vec![0, TRAIN, 0, 0],
        vec![REQUEST_MAGIC, 2, 0, 0],
        vec![REQUEST_MAGIC, TRAIN, 4, 0],
        vec![REQUEST_MAGIC, TRAIN, 0, 4],
        vec![REQUEST_MAGIC, PREDICT, 0, 1],
        vec![REQUEST_MAGIC, FINISH, 1, 0],
    ] {
        let mut model = VmModel::new().unwrap();
        let before = model.weights();
        let mut wire = Wire {
            input: Cursor::new(input),
            output: Vec::new(),
        };
        assert!(serve(&mut wire, &mut model).is_err());
        assert_eq!(before, model.weights());
        if !wire.output.is_empty() {
            assert_eq!(wire.output[1], 1);
        }
    }
}

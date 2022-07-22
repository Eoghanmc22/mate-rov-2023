use std::io;
use anyhow::Context;
use bincode::{config, Encode, Decode};
use bincode::error::DecodeError;

pub fn read<D: Decode, R: io::Read>(reader: &mut R) -> Option<anyhow::Result<D>> {
    match bincode::decode_from_std_read(reader, config::standard()) {
        Err(DecodeError::UnexpectedEnd) => {
            None
        }
        res => {
            Some(res.context("Could not read"))
        }
    }
}

pub fn write<S: Encode, W: io::Write>(data: &S, writer: &mut W) -> anyhow::Result<usize> {
    bincode::encode_into_std_write(data, writer, config::standard()).context("Could not write")
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek, SeekFrom};
    use crate::data::{read, write};
    use bincode::{Encode, Decode};

    #[test]
    fn read_one() {
        #[derive(Encode, Decode, PartialEq, Debug)]
        struct Test {
            value: u64,
            tuple: (i32, u8),
            float: f64,
            slice: Vec<u8>,
            unit: ()
        }

        // init test packet with random values
        let packet = Test {
            value: 10,
            tuple: (-37, 192),
            float: -32.73,
            slice: vec![2, 3, 5, 7, 11],
            unit: ()
        };

        // init test buffer
        let mut buffer = vec![];

        // write packet into buffer
        let amount = write(&packet, &mut buffer).expect("Write fail");

        // read packet back from buffer
        let read_packet = read(&mut &buffer[..amount]).expect("Read fail (ended)").expect("Read fail");

        // verify packet was read correctly
        assert_eq!(packet, read_packet);
    }

    #[test]
    fn read_five() {
        #[derive(Encode, Decode, PartialEq, Debug)]
        struct Test {
            value: u64,
            tuple: (i32, u8),
            float: f64,
            slice: Vec<u8>,
            unit: ()
        }

        // init test packets with random values
        let packet1 = Test {
            value: 10,
            tuple: (-37, 192),
            float: -32.73,
            slice: vec![2, 3, 5, 7, 11],
            unit: ()
        };
        let packet2 = Test {
            value: 20,
            tuple: (43, 0),
            float: 64.0,
            slice: vec![20, 30, 40],
            unit: ()
        };
        let packet3 = Test {
            value: 30,
            tuple: (23556, 255),
            float: 0.1,
            slice: vec![0],
            unit: ()
        };
        let packet4 = Test {
            value: 40,
            tuple: (-23543657, 22),
            float: -1.4,
            slice: vec![],
            unit: ()
        };
        let packet5 = Test {
            value: 50,
            tuple: (66, 5),
            float: 2034.6,
            slice: vec![41, 42, 43, 44],
            unit: ()
        };

        // init test buffer
        let mut buffer = Cursor::new(Vec::new());

        // write packets into buffer
        write(&packet1, &mut buffer).expect("Write fail");
        write(&packet2, &mut buffer).expect("Write fail");
        write(&packet3, &mut buffer).expect("Write fail");
        write(&packet4, &mut buffer).expect("Write fail");
        write(&packet5, &mut buffer).expect("Write fail");

        // Seek cursor back to the beginning of the buffer
        buffer.seek(SeekFrom::Start(0)).expect("Seek fail");

        // read packets back from buffer
        let read_packet1 = read(&mut buffer).expect("Read fail (ended)").expect("Read fail");
        let read_packet2 = read(&mut buffer).expect("Read fail (ended)").expect("Read fail");
        let read_packet3 = read(&mut buffer).expect("Read fail (ended)").expect("Read fail");
        let read_packet4 = read(&mut buffer).expect("Read fail (ended)").expect("Read fail");
        let read_packet5 = read(&mut buffer).expect("Read fail (ended)").expect("Read fail");

        // verify packets were read correctly
        assert_eq!(packet1, read_packet1);
        assert_eq!(packet2, read_packet2);
        assert_eq!(packet3, read_packet3);
        assert_eq!(packet4, read_packet4);
        assert_eq!(packet5, read_packet5);
    }
}

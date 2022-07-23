use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
use bincode::{Decode, Encode};
use mio::event::Event;
use crossbeam::channel::Receiver;
use anyhow::Context;
use crate::data;

const PROBE_LEN: usize = 4096;

pub fn handle_event<Out: Encode, In: Decode, Handler: FnMut(In), S: Read + Write>(
    event: &Event,
    connection: &mut S,
    packet_buffer: &mut Vec<u8>,
    read_buffer: &mut Vec<u8>,
    write_buffer: &mut Vec<u8>,
    packet_provider: &Receiver<Out>,
    packet_handler: &mut Handler,
    writeable: &mut bool,
    connected: &mut bool
) -> bool {
    if event.is_writable() {
        *connected = true;
        *writeable = true;

        if try_write(connection, packet_buffer, write_buffer, packet_provider, writeable, connected) {
            return true
        }
    }

    if event.is_readable() && *connected {
        if try_read(connection, read_buffer, packet_handler, connected) {
            return true
        }
    }

    false
}

pub fn try_write<Out: Encode, W: Write>(
    writer: &mut W,
    packet_buffer: &mut Vec<u8>,
    write_buffer: &mut Vec<u8>,
    packet_provider: &Receiver<Out>,
    writeable: &mut bool,
    connected: &bool
) -> bool {
    if !*connected || !*writeable {
        return false;
    }

    let (close, would_block) = write_remaining(writer, write_buffer);
    if close { return true }

    if !would_block {
        for packet in packet_provider.try_iter() {
            packet_buffer.clear();
            let amount = data::write(&packet, packet_buffer).unwrap();

            let (amount_written, should_close, would_block) = write(writer, &packet_buffer[..amount]);
            if should_close { return true }
            if amount_written < amount { write_buffer.write(&packet_buffer[amount_written..]).unwrap(); }

            if would_block {
                *writeable = false;
                break
            }
        }
    } else {
        *writeable = false;
    }

    false
}

// TODO improve
pub fn try_read<In: Decode, Handler: FnMut(In), R: Read>(
    reader: &mut R,
    read_buffer: &mut Vec<u8>,
    packet_handler: &mut Handler,
    connected: &mut bool
) -> bool {
    if !*connected {
        return false;
    }

    loop {
        let (_amount_read, should_close, would_block) = read(reader, read_buffer);
        if should_close { return true }
        if would_block { break }

        let mut reader = Cursor::new(&read_buffer[..]);
        let mut last_safe = 0;
        loop {
            match data::read(&mut reader) {
                Some(Ok(packet)) => {
                    (packet_handler)(packet);
                    last_safe = reader.position()
                },
                Some(Err(err)) => {
                    println!("parse error: {}", err);
                    return true
                },
                None => {
                    if last_safe != 0 {
                        read_buffer.drain(..last_safe);
                    }
                    break
                }
            }
        }
    }

    false
}

fn write_remaining<W: Write>(writer: &mut W, write_buffer: &mut Vec<u8>) -> (bool, bool) { // close, would block
    if write_buffer.is_empty() { return (false, false) }

    let (amount_written, should_close, would_block) = write(writer, &write_buffer[..]);
    if should_close { return (true, false) }

    write_buffer.drain(..amount_written);

    (false, would_block)
}

fn write<W: Write>(writer: &mut W, mut data: &[u8]) -> (usize, bool, bool) { // amount, should close, would block
    let start_len = data.len();

    while !data.is_empty() {
        match writer.write(data) {
            Ok(0) => return (0, true, false),
            Ok(amt) => data = &data[amt..],
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return (start_len - data.len(), false, true),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(err) => {
                println!("write error: {}", err);
                return (0, true, false)
            }
        }
    }

    (start_len, false, false)
}

// TODO improve
fn read<R: Read>(reader: &mut R, read_buffer: &mut Vec<u8>) -> (usize, bool, bool) { // amount, should close, would block
    let mut probe = [0u8; PROBE_LEN];

    loop {
        match reader.read(&mut probe[..]) {
            Ok(0) => return (0, true, false),
            Ok(amt) => {
                read_buffer.write(&probe[..amt]).unwrap();
                return (amt, false, false)
            }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return (0, false, true),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(err) => {
                println!("read error: {:?}", err);
                return (0, true, false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Error, ErrorKind, Read, Seek, SeekFrom, Write};
    use crate::net::{read, write};

    // constant stuff
    const LEN: usize = 255;
    const APPEND: usize = 25;
    const APPEND_VAL: u8 = 5;

    struct EOF2WouldBlock(Cursor<Vec<u8>>);
    impl Read for EOF2WouldBlock {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.0.position() as usize >= self.0.get_ref().len() {
                return Err(Error::new(ErrorKind::WouldBlock, "would block"));
            }

            self.0.read(buf)
        }
    }


    #[test]
    fn write_read() {
        // setup write
        let buffer = (0u8..).take(LEN).collect::<Vec<u8>>();
        let mut writer = Cursor::new(Vec::new());

        // write
        let (amt, should_close, would_block) = write(&mut writer, &buffer[..]);
        assert_eq!(amt, LEN);
        assert_eq!(should_close, false);
        assert_eq!(would_block, false);


        // setup read
        writer.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = EOF2WouldBlock(writer);
        let mut read_buffer = Cursor::new(Vec::new());

        // read
        let (amt, should_close) = read(&mut reader, &mut read_buffer);
        assert_eq!(amt, LEN);
        assert_eq!(should_close, false);

        // verify
        assert_eq!(&buffer, read_buffer.get_ref());
    }

    #[test]
    fn write_read_append() {
        // setup write
        let buffer = (0u8..).take(LEN).collect::<Vec<u8>>();
        let mut writer = Cursor::new(Vec::new());

        // write
        let (amt, should_close, would_block) = write(&mut writer, &buffer[..]);
        assert_eq!(amt, LEN);
        assert_eq!(should_close, false);
        assert_eq!(would_block, false);


        // setup read
        writer.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = EOF2WouldBlock(writer);
        let mut read_buffer = Vec::new();

        // append
        read_buffer.write_all(&[APPEND_VAL; APPEND]).unwrap();

        // read
        let (amt, should_close) = read(&mut reader, &mut read_buffer);
        assert_eq!(amt, LEN);
        assert_eq!(should_close, false);

        // verify
        assert_eq!(&[APPEND_VAL; APPEND], &read_buffer.get_ref()[..APPEND]);
        assert_eq!(&buffer, &read_buffer.get_ref()[APPEND..]);
    }
}

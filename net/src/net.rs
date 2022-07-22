use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
use bincode::{Decode, Encode};
use mio::event::Event;
use crossbeam::channel::Receiver;
use anyhow::Context;
use crate::data;

pub fn handle_event<Out: Encode, In: Decode, Handler: FnMut(In), S: Read + Write>(
    event: &Event,
    connection: &mut S,
    packet_buffer: &mut Vec<u8>,
    read_buffer: &mut Cursor<Vec<u8>>,
    write_buffer: &mut Cursor<Vec<u8>>,
    packet_provider: &Receiver<Out>,
    packet_handler: &mut Handler,
    writeable: &mut bool,
    connected: &mut bool
) -> anyhow::Result<bool> {
    if event.is_writable() {
        *connected = true;
        *writeable = true;

        if try_write(connection, packet_buffer, write_buffer, packet_provider, writeable, connected).context("try_write")? {
            return Ok(true)
        }
    }

    if event.is_readable() && *connected {
        if try_read(connection, read_buffer, packet_handler, connected).context("try_write")? {
            return Ok(true)
        }
    }

    Ok(false)
}

pub fn try_write<Out: Encode, S: Read + Write>(
    connection: &mut S,
    packet_buffer: &mut Vec<u8>,
    write_buffer: &mut Cursor<Vec<u8>>,
    packet_provider: &Receiver<Out>,
    writeable: &mut bool,
    connected: &bool
) -> anyhow::Result<bool> {
    if !*connected || !*writeable {
        return Ok(false);
    }

    let (close, would_block) = write_remaining(connection, write_buffer).context("write_remaining")?;
    if close { return Ok(true) }

    if !would_block {
        for packet in packet_provider.try_iter() {
            packet_buffer.clear();
            let amount = data::write(&packet, packet_buffer).unwrap();

            let (amount_written, would_block) = write(connection, &packet_buffer[..amount]).context("write")?;
            if amount_written == 0 { return Ok(true) }
            if amount_written != amount { write_buffer.write(&packet_buffer[amount_written..]).unwrap(); }

            if would_block {
                *writeable = false;
                break
            }
        }
    }

    Ok(false)
}

pub fn try_read<In: Decode, Handler: FnMut(In), S: Read + Write>(
    connection: &mut S,
    read_buffer: &mut Cursor<Vec<u8>>,
    packet_handler: &mut Handler,
    connected: &mut bool
) -> anyhow::Result<bool> {
    if !*connected {
        return Ok(false);
    }

    let amount_read = read(connection, read_buffer).context("read")?;
    if amount_read == 0 { return Ok(true) }

    let max_pos = read_buffer.position() as usize;
    let mut reader = Cursor::new(&read_buffer.get_ref()[..max_pos]);

    loop {
        match data::read(&mut reader) {
            Some(Ok(packet)) => (packet_handler)(packet),
            Some(Err(err)) => return Err(err).context("parse"),
            None => {
                let position = reader.position();
                read_buffer.get_mut().copy_within(position as usize.., 0);
                read_buffer.seek(SeekFrom::Current(-(position as i64))).expect("seek");
                break
            }
        }
    }

    Ok(false)
}

fn write_remaining<W: Write>(writer: &mut W, write_buffer: &mut Cursor<Vec<u8>>) -> anyhow::Result<(bool, bool)> { // close, would block
    let cursor = write_buffer.position() as usize;
    if cursor == 0 { return Ok((false, false)) }

    let (amount_written, would_block) = write(writer, &write_buffer.get_ref()[..cursor]).context("write")?;
    if amount_written == 0 { return Ok((true, true)) }

    write_buffer.get_mut().copy_within(amount_written.., 0);
    write_buffer.seek(SeekFrom::Current(-(amount_written as i64))).expect("seek");

    Ok((false, would_block))
}

fn write<W: Write>(writer: &mut W, mut data: &[u8]) -> anyhow::Result<(usize, bool)> { // amount (0 for should close), would block
    let start_len = data.len();

    while !data.is_empty() {
        match writer.write(data) {
            Ok(0) => return Ok((0, true)),
            Ok(amt) => data = &data[amt..],
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok((start_len - data.len(), true)),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err).context("Write")
        }
    }

    Ok((start_len, false))
}

// TODO improve
fn read<R: Read>(reader: &mut R, read_buffer: &mut Cursor<Vec<u8>>) -> anyhow::Result<usize> { // amount (0 for should close), would block
    let mut probe = [0u8; 128];
    let start_pos = read_buffer.position();

    loop {
        match reader.read(&mut probe[..]) {
            Ok(0) => return Ok(0),
            Ok(amt) => { read_buffer.write(&probe[..amt]).unwrap(); }
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok((read_buffer.position() - start_pos) as usize),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Err(err).context("Read")
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
        let (amt, would_block) = write(&mut writer, &buffer[..]).unwrap();
        assert_eq!(amt, LEN);
        assert_eq!(would_block, false);


        // setup read
        writer.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = EOF2WouldBlock(writer);
        let mut read_buffer = Cursor::new(Vec::new());

        // read
        let amt = read(&mut reader, &mut read_buffer).unwrap();
        assert_eq!(amt, LEN);

        // verify
        assert_eq!(&buffer, read_buffer.get_ref());
    }

    #[test]
    fn write_read_append() {
        // setup write
        let buffer = (0u8..).take(LEN).collect::<Vec<u8>>();
        let mut writer = Cursor::new(Vec::new());

        // write
        let (amt, would_block) = write(&mut writer, &buffer[..]).unwrap();
        assert_eq!(amt, LEN);
        assert_eq!(would_block, false);


        // setup read
        writer.seek(SeekFrom::Start(0)).unwrap();
        let mut reader = EOF2WouldBlock(writer);
        let mut read_buffer = Cursor::new(Vec::new());

        // append
        read_buffer.write_all(&[APPEND_VAL; APPEND]).unwrap();

        // read
        let amt = read(&mut reader, &mut read_buffer).unwrap();
        assert_eq!(amt, LEN);

        // verify
        assert_eq!(&[APPEND_VAL; APPEND], &read_buffer.get_ref()[..APPEND]);
        assert_eq!(&buffer, &read_buffer.get_ref()[APPEND..]);
    }
}

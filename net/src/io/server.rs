use std::{io, thread};
use std::io::{Cursor, ErrorKind, Read, ReadBuf, Seek, SeekFrom, Write};
use std::mem::MaybeUninit;
use std::net::SocketAddr;
use anyhow::{bail, Context};
use bincode::{Decode, Encode};
use crossbeam::channel::{bounded, Receiver, Sender};
use mio::{Events, Interest, Poll, Token};
use mio::event::Event;
use mio::net::{TcpListener, TcpStream};
use crate::io::data;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

pub fn start_server<Out: Encode, In: Decode, Handler: FnMut(In)>(addr: SocketAddr, packet_handler: Handler) -> Sender<Out> {
    let (packet_producer, packet_provider) = bounded(25);

    thread::spawn(move || {
        server(addr, packet_provider, packet_handler).unwrap();
    });

    packet_producer
}

fn server<Out: Encode, In: Decode, Handler: FnMut(In)>(addr: SocketAddr, packet_provider: Receiver<Out>, mut packet_handler: Handler) -> anyhow::Result<()> {
    let mut poll = Poll::new().context("Could not create poll")?;
    let mut events = Events::with_capacity(128);
    let mut server = TcpListener::bind(addr).context("Could not bind listener")?;

    poll.registry().register(&mut server, SERVER, Interest::READABLE).context("Could not register")?;

    let mut client = None;

    let mut packet_buffer = Vec::new();
    let mut read_buffer = Cursor::new(Vec::new());
    let mut write_buffer = Cursor::new(Vec::new());

    let mut writable = false;
    let mut connected = false;

    //TODO Compression?

    loop {
        poll.poll(&mut events, None).context("Could not poll")?;

        for event in &events {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => bail!(e)
                    };

                    if let Some(_) = CLIENT {
                        drop(connection);
                        continue;
                    }

                    println!("Accepted connection from: {}", address);

                    poll.registry().register(
                        &mut connection,
                        CLIENT,
                        Interest::READABLE | Interest::WRITABLE
                    )?;

                    client = Some(connection);
                }
                CLIENT => {
                    let connection = client.as_mut().unwrap();

                    if handle_event(event, connection, &mut packet_buffer, &mut read_buffer, &mut write_buffer, &packet_provider, &mut packet_handler, &mut writable, &mut connected) {
                        poll.registry().deregister(connection)?;
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_event<Out: Encode, In: Decode, Handler: FnMut(In)>(
    event: &Event,
    connection: &mut TcpStream,
    packet_buffer: &mut Vec<u8>,
    read_buffer: &mut Cursor<Vec<u8>>,
    write_buffer: &mut Cursor<Vec<u8>>,
    packet_provider: &Receiver<Out>,
    packet_handler: &mut Handler,
    writeable: &mut bool,
    connected: &mut bool
) -> bool {
    if event.is_writable() {
        *connected = true;
        *writeable = true;

        let (close, would_block) = write_remaining(connection, write_buffer).context("write_remaining")?;
        if close { return true }

        if !would_block {
            for packet in &packet_provider {
                packet_buffer.clear();
                let amount = data::write(&packet, packet_buffer).unwrap();

                let (amount_written, would_block) = write(connection, &packet_buffer[..amount]).context("write")?;
                if amount_written == 0 { return true }
                if amount_written != amount { write_buffer.write(&packet_buffer[amount_written..]).unwrap() }

                if would_block {
                    *writeable = false;
                    break
                }
            }
        }
    }

    if event.is_readable() && *connected {
        let amount_read = read(connection, read_buffer).context("read");
        if amount_read == 0 { return true }

        let max_pos = read_buffer.position() as usize;
        let mut reader = Cursor::new(&read_buffer.get_ref()[..max_pos]);

        loop {
            match data::read(&mut reader) {
                Some(Ok(packet)) => (packet_handler)(packet),
                Some(res) => res.context("parse")?,
                None => {
                    read_buffer.get_mut().copy_within(reader.position().., 0);
                    read_buffer.seek(SeekFrom::Current(-reader.position() as i64)).expect("seek");
                    break
                }
            }
        }
    }

    false
}

fn write_remaining(connection: &mut TcpStream, write_buffer: &mut Cursor<Vec<u8>>) -> anyhow::Result<(bool, bool)> { // close, would block
    let cursor = write_buffer.position();

    let (amount_written, would_block) = write(connection, &write_buffer.get_ref()[..cursor]).context("write")?;
    if amount_written == 0 { return Ok((true, true)) }

    write_buffer.get_mut().copy_within(amount_written.., 0);
    write_buffer.seek(SeekFrom::Current(-amount_written as i64)).expect("seek");

    Ok((false, would_block))
}

fn write(connection: &mut TcpStream, mut data: &[u8]) -> anyhow::Result<(usize, bool)> { // amount (0 for should close), would block
    let start_len = data.len();

    while !data.is_empty() {
        match connection.write(data) {
            Ok(0) => return Ok((0, true)),
            Ok(amt) => data = *data[amt..],
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok((start_len - data.len(), true)),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            err => err.with_context("Write")?
        }
    }

    Ok((start_len, false))
}

// TODO improve
fn read(connection: &mut TcpStream, read_buffer: &mut Cursor<Vec<u8>>) -> anyhow::Result<usize> { // amount (0 for should close), would block
    let mut probe = [0u8; 128];
    let start_pos = read_buffer.position();

    loop {
        match connection.read(&mut probe[..]) {
            Ok(0) => return Ok(0),
            Ok(amt) => read_buffer.write(&probe[..amt]).unwrap(),
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => return Ok((read_buffer.position() - start_pos) as usize),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            err => err.with_context("Read")?
        };
    }
}

use std::thread;
use std::io::{Cursor, ErrorKind};
use std::net::SocketAddr;
use anyhow::{bail, Context};
use bincode::{Decode, Encode};
use crossbeam::channel::{bounded, Receiver, Sender};
use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use crate::io::net;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

pub fn start_server<Out: Encode + Send + 'static, In: Decode, Handler: FnMut(In) + Send + 'static>(addr: SocketAddr, packet_handler: Handler) -> Sender<Out> {
    let (packet_producer, packet_provider) = bounded(25);

    thread::spawn(move || {
        server(addr, packet_provider, packet_handler).unwrap();
    });

    packet_producer
}

fn server<Out: Encode, In: Decode, Handler: FnMut(In)>(addr: SocketAddr, packet_provider: Receiver<Out>, mut packet_handler: Handler) -> anyhow::Result<()> {
    let mut poll = Poll::new().context("Create poll")?;
    let mut events = Events::with_capacity(128);
    let mut server = TcpListener::bind(addr).context("Bind listener")?;

    poll.registry().register(&mut server, SERVER, Interest::READABLE).context("Register")?;

    let mut client = None;

    let mut packet_buffer = Vec::new();
    let mut read_buffer = Cursor::new(Vec::new());
    let mut write_buffer = Cursor::new(Vec::new());

    let mut writable = false;
    let mut connected = false;

    //TODO Compression?

    loop {
        poll.poll(&mut events, None).context("Poll")?;

        for event in &events {
            match event.token() {
                SERVER => loop {
                    let (mut connection, address) = match server.accept() {
                        Ok((connection, address)) => (connection, address),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => bail!(e)
                    };

                    if let Some(_) = client {
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

                    if net::handle_event(event, connection, &mut packet_buffer, &mut read_buffer, &mut write_buffer, &packet_provider, &mut packet_handler, &mut writable, &mut connected).context("handle event")? {
                        poll.registry().deregister(connection)?;
                    }
                }
                _ => {}
            }
        }
    }
}

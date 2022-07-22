use std::thread;
use std::io::{Cursor, ErrorKind};
use std::net::SocketAddr;
use std::time::Duration;
use anyhow::{bail, Context};
use bincode::{Decode, Encode};
use crossbeam::channel::{Receiver, Sender, unbounded};
use mio::{Events, Interest, Poll, Token};
use mio::net::TcpListener;
use crate::net;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

pub fn start_server<Out: Encode + Send + 'static, In: Decode, Handler: FnMut(In, &Sender<Out>) + Send + 'static>(addr: SocketAddr, mut packet_handler: Handler) -> Sender<Out> {
    let (packet_producer, packet_provider) = unbounded();

    {
        let packet_producer = packet_producer.clone();
        thread::spawn(move || {
            server(addr, packet_provider, |packet| (packet_handler)(packet, &packet_producer)).unwrap();
        });
    }

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
        poll.poll(&mut events, Some(Duration::from_millis(1))).context("Poll")?;

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

                    if net::handle_event(event, connection, &mut packet_buffer, &mut read_buffer, &mut write_buffer, &packet_provider, &mut packet_handler, &mut writable, &mut connected) {
                        poll.registry().deregister(connection)?;
                        writable = false;
                        connected = false;
                        client = None;
                        read_buffer.set_position(0);
                        write_buffer.set_position(0);
                    }
                }
                _ => {}
            }
        }

        if let Some(ref mut connection) = client {
            if net::try_write(connection, &mut packet_buffer, &mut write_buffer, &packet_provider, &mut writable, &mut connected) {
                poll.registry().deregister(connection)?;
                writable = false;
                connected = false;
                client = None;
                read_buffer.set_position(0);
                write_buffer.set_position(0);
            }
        }
    }
}

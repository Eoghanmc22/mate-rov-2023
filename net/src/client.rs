use std::thread;
use std::io::Cursor;
use std::net::SocketAddr;
use std::time::Duration;
use anyhow::Context;
use bincode::{Decode, Encode};
use crossbeam::channel::{Receiver, Sender, unbounded};
use mio::{Events, Interest, Poll, Token};
use mio::net::TcpStream;
use crate::net;

const CONNECTION: Token = Token(0);

pub fn start_client<Out: Encode + Send + 'static, In: Decode, Handler: FnMut(In, &Sender<Out>) + Send + 'static>(addr: SocketAddr, mut packet_handler: Handler) -> Sender<Out> {
    let (packet_producer, packet_provider) = unbounded();

    {
        let packet_producer = packet_producer.clone();
        thread::spawn(move || {
            client(addr, packet_provider, |packet| (packet_handler)(packet, &packet_producer)).unwrap();
        });
    }

    packet_producer
}

fn client<Out: Encode, In: Decode, Handler: FnMut(In)>(addr: SocketAddr, packet_provider: Receiver<Out>, mut packet_handler: Handler) -> anyhow::Result<()> {
    let mut poll = Poll::new().context("Create poll")?;
    let mut events = Events::with_capacity(128);
    let mut connection = TcpStream::connect(addr).context("Connect to server")?;

    poll.registry().register(&mut connection, CONNECTION, Interest::READABLE | Interest::WRITABLE).context("Register")?;

    let mut packet_buffer = Vec::new();
    let mut read_buffer = Cursor::new(Vec::new());
    let mut write_buffer = Cursor::new(Vec::new());

    let mut writable = false;
    let mut connected = false;

    //TODO Compression?

    loop {
        poll.poll(&mut events, Some(Duration::from_millis(1))).context("Could not poll")?;

        for event in &events {
            match event.token() {
                CONNECTION => {
                    if net::handle_event(event, &mut connection, &mut packet_buffer, &mut read_buffer, &mut write_buffer, &packet_provider, &mut packet_handler, &mut writable, &mut connected).context("handle event")? {
                        poll.registry().deregister(&mut connection)?;
                        writable = false;
                        connected = false;
                    }
                }
                _ => {}
            }
        }

        if net::try_write(&mut connection, &mut packet_buffer, &mut write_buffer, &packet_provider, &mut writable, &mut connected).context("handle event")? {
            poll.registry().deregister(&mut connection)?;
            writable = false;
            connected = false;
        }
    }
}
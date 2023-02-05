use std::{
    io::{self, ErrorKind, Read, Write},
    net::SocketAddr,
    sync::{
        atomic::{self, AtomicUsize},
        Arc,
    },
};

use crossbeam::channel::{self, Receiver, Sender};
use fxhash::FxHashMap as HashMap;
use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token, Waker,
};
use thiserror::Error;

const WAKER_TOKEN: Token = Token(0);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
pub struct Networking<P> {
    poll: Poll,
    waker: Arc<Waker>,
    queue: (Sender<Message<P>>, Receiver<Message<P>>),
}

impl<P: Packet> Networking<P> {
    pub fn new() -> NetResult<Self> {
        let poll = Poll::new()?;

        let waker = Waker::new(poll.registry(), WAKER_TOKEN)?;
        let waker = Arc::new(waker);

        let queue = channel::bounded(50);

        Ok(Networking { poll, waker, queue })
    }

    pub fn messenger(&self) -> Messenger<P> {
        Messenger {
            waker: self.waker.clone(),
            sender: self.queue.0.clone(),
        }
    }

    pub fn start(self, handler: impl FnMut(Event<P>)) {
        let Networking { poll, waker, queue } = self;

        start_worker(poll, queue.1, handler);
    }
}

enum Event<P> {
    Conected(Token, SocketAddr),
    Accepted(Token /* TODO */),

    Data(Token, P),

    ConnectionFailed(/* TODO */),
    Error(NetError),
    // TODO
}

enum Message<P> {
    Connect(SocketAddr),
    Bind(SocketAddr),
    Disconect(Token),
    Packet(Token, P),
    PacketBrodcast(P),
    Shutdown,
}

pub struct Messenger<P> {
    waker: Arc<Waker>,
    sender: Sender<Message<P>>,
}

impl<P> Messenger<P> {
    pub fn send_packet(&self, peer: Token, packet: P) -> Result<(), MessageError> {
        let message = Message::Packet(peer, packet);

        self.send_message(message)
    }

    pub fn brodcast_packet(&self, packet: P) -> Result<(), MessageError> {
        let message = Message::PacketBrodcast(packet);

        self.send_message(message)
    }

    pub fn connect_to(&self, peer: SocketAddr) -> Result<(), MessageError> {
        let message = Message::Connect(peer);

        self.send_message(message)
    }

    pub fn bind_at(&self, addr: SocketAddr) -> Result<(), MessageError> {
        let message = Message::Bind(addr);

        self.send_message(message)
    }

    pub fn shutdown(&self) -> Result<(), MessageError> {
        let message = Message::Shutdown;

        self.send_message(message)
    }

    fn send_message(&self, message: Message<P>) -> Result<(), MessageError> {
        self.sender.send(message).map_err(|_| MessageError)?;
        self.waker.wake().map_err(|_| MessageError)
    }
}

pub trait Packet: Clone {
    fn to_network_data(self) -> Vec<u8>;
    fn from_network_data(data: &[u8]) -> Self;
}

pub type NetResult<T> = Result<T, NetError>;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Messenging Error: {0}")]
    Message(#[from] MessageError),
    #[error("Tried to send packet to unknown peer: {0:?}")]
    UnknownPeer(Token),
}

#[derive(Error, Debug, Default)]
#[error("Failed to send message to worker")]
pub struct MessageError;

// TODO More detailed errors
// TODO Add logs
fn start_worker<P: Packet>(
    mut poll: Poll,
    receiver: Receiver<Message<P>>,
    mut handler: impl FnMut(Event<P>),
) {
    let mut peers = HashMap::default();
    let mut accptors = HashMap::default();

    let mut events = Events::with_capacity(100);

    'outer: loop {
        let res = poll.poll(&mut events, None);

        if let Err(err) = res {
            (handler)(Event::Error(err.into()));
        }

        for event in &events {
            if event.token() == WAKER_TOKEN {
                for message in receiver.try_iter() {
                    match message {
                        Message::Connect(peer) => {
                            let token = NEXT_TOKEN.fetch_add(1, atomic::Ordering::Relaxed);
                            let token = Token(token);

                            let socket = TcpStream::connect(peer);
                            let mut socket = match socket {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(err.into()));
                                    continue;
                                }
                            };

                            let res = poll.registry().register(
                                &mut socket,
                                token,
                                Interest::READABLE | Interest::WRITABLE,
                            );
                            if let Err(err) = res {
                                (handler)(Event::Error(err.into()));
                                continue;
                            }

                            peers.insert(
                                token,
                                Peer {
                                    conected: false,
                                    socket,
                                },
                            );
                        }
                        Message::Bind(addr) => {
                            let token = NEXT_TOKEN.fetch_add(1, atomic::Ordering::Relaxed);
                            let token = Token(token);

                            let listener = TcpListener::bind(addr);
                            let mut listener = match listener {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(err.into()));
                                    continue;
                                }
                            };

                            let res =
                                poll.registry()
                                    .register(&mut listener, token, Interest::READABLE);
                            if let Err(err) = res {
                                (handler)(Event::Error(err.into()));
                                continue;
                            }

                            accptors.insert(token, Acceptor { listener });
                        }
                        Message::Disconect(token) => {
                            peers.remove(&token);
                            accptors.remove(&token);
                        }
                        Message::Packet(peer_token, packet) => {
                            if let Some(peer) = peers.get_mut(&peer_token) {
                                let res = peer.write_packet(packet);
                                if let Err(err) = res {
                                    (handler)(Event::Error(err.into()));
                                    peers.remove(&peer_token);
                                }
                            } else {
                                (handler)(Event::Error(NetError::UnknownPeer(peer_token)));
                            }
                        }
                        Message::PacketBrodcast(packet) => {
                            let mut to_remove = Vec::new();

                            for (token, peer) in &mut peers {
                                let res = peer.write_packet(packet.clone());
                                if let Err(err) = res {
                                    (handler)(Event::Error(err.into()));
                                    to_remove.push(*token);
                                    continue;
                                }
                            }

                            for token in to_remove {
                                peers.remove(&token);
                            }
                        }
                        Message::Shutdown => {
                            break 'outer;
                        }
                    }
                }
            } else if let Some(peer) = peers.get_mut(&event.token()) {
                if !peer.conected {
                    if event.is_writable() {
                        match peer.socket.peer_addr() {
                            Ok(_) => {
                                let res = peer.connect();
                                if let Err(err) = res {
                                    (handler)(Event::Error(err.into()));
                                    peers.remove(&event.token());
                                    continue;
                                }
                            }
                            Err(err) if err.kind() == ErrorKind::NotConnected => {
                                continue;
                            }
                            Err(err) => {
                                (handler)(Event::Error(err.into()));
                                peers.remove(&event.token());
                                continue;
                            }
                        }
                    } else {
                        continue;
                    }
                }

                if event.is_writable() {
                    let res = peer.write_remaining();
                    if let Err(err) = res {
                        (handler)(Event::Error(err.into()));
                        peers.remove(&event.token());
                        continue;
                    }
                }

                if event.is_readable() {
                    todo!("Read Packets");
                }
            } else if let Some(acceptor) = accptors.get_mut(&event.token()) {
                todo!("Accept peers")
            } else {
                // TODO log
            }
        }
    }
}

struct Peer<S> {
    conected: bool,
    socket: S,
}

impl Peer<TcpStream> {
    fn connect(&mut self) -> NetResult<()> {
        self.conected = true;
        self.socket.set_nodelay(true)?;

        todo!()
    }
}

impl<S: Write> Peer<S> {
    fn write_packet<P: Packet>(&mut self, packet: P) -> NetResult<()> {
        todo!()
    }

    fn write_remaining(&mut self) -> NetResult<()> {
        todo!()
    }
}

impl<S: Read> Peer<S> {
    fn read_packet<P: Packet>(&mut self) -> NetResult<Option<P>> {
        todo!()
    }
}

struct Acceptor<L> {
    listener: L,
}

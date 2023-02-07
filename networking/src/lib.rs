#![feature(split_array)]

pub mod buf;

use std::{
    io::{self, ErrorKind, Read, Write},
    mem,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use buf::Buffer;
use crossbeam::channel::{self, Receiver, Sender};
use fxhash::FxHashMap as HashMap;
use mio::{
    net::{TcpListener, TcpStream},
    Events, Interest, Poll, Token, Waker,
};
use thiserror::Error;
use tracing::{error, span, warn, Level};

const WAKER_TOKEN: Token = Token(0);

static NEXT_TOKEN: AtomicUsize = AtomicUsize::new(1);

const PROBE_LENGTH: usize = 4096;

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
        let _ = waker;

        start_worker(poll, queue.1, handler);
    }
}

pub enum Event<P> {
    Conected(Token, SocketAddr),
    Accepted(Token, SocketAddr),

    Data(Token, P),

    Error(Option<Token>, NetError),
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

    pub fn disconnect(&self, peer: Token) -> Result<(), MessageError> {
        let message = Message::Disconect(peer);

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
    fn expected_size(&self) -> usize;
    fn write_buf(self, buffer: &mut [u8]) -> &mut [u8];
    fn read_buf(buffer: &mut &[u8]) -> anyhow::Result<Self>;
}

pub type NetResult<T> = Result<T, NetError>;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("IO Error: {0}")]
    Io(#[from] io::Error),
    #[error("Peer closed socket")]
    PeerClosed,
    #[error("Tried to write packet with len {0} which does not fit in header")]
    OversizedPacket(usize),
    #[error("Messenging Error: {0}")]
    Message(#[from] MessageError),
    #[error("Tried to send packet to unknown peer: {0:?}")]
    UnknownPeer(Token),
    #[error("Could not parse packet: {0}")]
    ParsingError(anyhow::Error),
    #[error("Error {0}: Caused by: ({1})")]
    Chain(String, #[source] Box<NetError>),
}

impl NetError {
    pub fn chain(self, message: String) -> Self {
        NetError::Chain(message, Box::new(self))
    }
}

#[derive(Error, Debug, Default)]
#[error("Failed to send message to worker")]
pub struct MessageError;

fn start_worker<P: Packet>(
    mut poll: Poll,
    receiver: Receiver<Message<P>>,
    mut handler: impl FnMut(Event<P>),
) {
    span!(Level::INFO, "Network Worker Thread");

    let mut peers = HashMap::default();
    let mut accptors = HashMap::default();
    let mut temp_buf = Buffer::with_capacity(PROBE_LENGTH * 2);

    let mut events = Events::with_capacity(100);

    'outer: loop {
        let res = poll.poll(&mut events, None);

        if let Err(err) = res {
            error!("Could not poll, sleeping 300ms");
            (handler)(Event::Error(None, err.into()));

            // Slight cool down to avoid a possible error spam
            thread::sleep(Duration::from_millis(300));
            continue 'outer;
        }

        'event: for event in &events {
            if event.token() == WAKER_TOKEN {
                // Handle incomming Message events
                'message: for message in receiver.try_iter() {
                    match message {
                        Message::Connect(peer) => {
                            // Create socket
                            let res = TcpStream::connect(peer);
                            let mut socket = match res {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(
                                        None,
                                        NetError::from(err).chain("Connect to peer".to_owned()),
                                    ));
                                    continue 'message;
                                }
                            };

                            // Assign Token
                            let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                            let token = Token(token);

                            // Register event intreast
                            let res = poll.registry().register(
                                &mut socket,
                                token,
                                Interest::READABLE | Interest::WRITABLE,
                            );
                            if let Err(err) = res {
                                (handler)(Event::Error(
                                    Some(token),
                                    NetError::from(err).chain("Register socket".to_owned()),
                                ));
                                continue 'message;
                            }

                            let peer = Peer::new(socket);

                            // Register peer
                            peers.insert(token, peer);
                        }
                        Message::Bind(addr) => {
                            // Create listner
                            let listener = TcpListener::bind(addr);
                            let mut listener = match listener {
                                Ok(socket) => socket,
                                Err(err) => {
                                    (handler)(Event::Error(
                                        None,
                                        NetError::from(err).chain("Bind listner".to_owned()),
                                    ));
                                    continue 'message;
                                }
                            };

                            // Assign token
                            let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                            let token = Token(token);

                            // Register event intreast
                            let res =
                                poll.registry()
                                    .register(&mut listener, token, Interest::READABLE);
                            if let Err(err) = res {
                                (handler)(Event::Error(
                                    Some(token),
                                    NetError::from(err).chain("Register listner".to_owned()),
                                ));
                                continue 'message;
                            }

                            // Register acceptor
                            accptors.insert(token, Acceptor { listener });
                        }
                        Message::Disconect(token) => {
                            peers.remove(&token);
                            accptors.remove(&token);
                        }
                        Message::Packet(peer_token, packet) => {
                            // Lookup peer and send packet
                            if let Some(peer) = peers.get_mut(&peer_token) {
                                let res = peer.write_packet(packet, &mut temp_buf);
                                if let Err(err) = res {
                                    (handler)(Event::Error(
                                        Some(peer_token),
                                        NetError::from(err).chain("Write packet".to_owned()),
                                    ));
                                    peers.remove(&peer_token);
                                    continue 'message;
                                }
                            } else {
                                // Handle peer not found
                                (handler)(Event::Error(
                                    None,
                                    NetError::UnknownPeer(peer_token)
                                        .chain("Write packet".to_owned()),
                                ));
                                continue 'message;
                            }
                        }
                        Message::PacketBrodcast(packet) => {
                            let mut to_remove = Vec::new();

                            // Send packet to every peer
                            'peer: for (token, peer) in &mut peers {
                                let res = peer.write_packet(packet.clone(), &mut temp_buf);
                                if let Err(err) = res {
                                    (handler)(Event::Error(
                                        Some(*token),
                                        NetError::from(err).chain("Brodcast packet".to_owned()),
                                    ));
                                    to_remove.push(*token);
                                    continue 'peer;
                                }
                            }

                            // Remove peers that errored
                            // Needed to bypass lifetime issues
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
                // Peers don't connect isntantly
                // Set up the socket if the peer just connected
                // else ignore events for unconected peers
                if !peer.conected {
                    if event.is_writable() {
                        match peer.socket.peer_addr() {
                            Ok(addr) => {
                                let res = peer.connect();
                                match res {
                                    Ok(()) => {
                                        (handler)(Event::Conected(event.token(), addr));
                                        // Happy path
                                    }
                                    Err(err) => {
                                        // Couldnt setup the peer's socket
                                        (handler)(Event::Error(
                                            Some(event.token()),
                                            NetError::from(err)
                                                .chain("Setup peer socket".to_owned()),
                                        ));
                                        peers.remove(&event.token());
                                        continue 'event;
                                    }
                                }
                            }
                            Err(err) if err.kind() == ErrorKind::NotConnected => {
                                // Try again on the next event
                                continue 'event;
                            }
                            Err(err) => {
                                // Couldnt connect for whatever reason
                                (handler)(Event::Error(
                                    Some(event.token()),
                                    NetError::from(err).chain("Connect to peer".to_owned()),
                                ));
                                peers.remove(&event.token());
                                continue 'event;
                            }
                        }
                    } else {
                        // Shouldn't be hit but this is not guranetted
                        // Ignore false event
                        continue 'event;
                    }
                }

                // Handle the socket being newly writeable
                if event.is_writable() {
                    // Write any buffered packets
                    // Also marks peer as writeable if it preaviously wasnt
                    let res = peer.write_remaining();
                    if let Err(err) = res {
                        (handler)(Event::Error(
                            Some(event.token()),
                            NetError::from(err).chain("Write packets".to_owned()),
                        ));
                        peers.remove(&event.token());
                        continue 'event;
                    }
                }

                // Handle the socket being newly readable
                if event.is_readable() {
                    // Read all incomming packets from peer
                    'packets: loop {
                        let res = peer.read_packet(&mut temp_buf);
                        match res {
                            Ok(Some(packet)) => {
                                (handler)(Event::Data(event.token(), packet));
                            }
                            Ok(None) => {
                                break 'packets;
                            }
                            Err(err) => {
                                (handler)(Event::Error(
                                    Some(event.token()),
                                    NetError::from(err).chain("Read packets".to_owned()),
                                ));
                                peers.remove(&event.token());
                                continue 'event;
                            }
                        }
                    }
                }
            } else if let Some(acceptor) = accptors.get_mut(&event.token()) {
                if event.is_readable() {
                    // Accept all new connections
                    'accept: loop {
                        // Create socket
                        let res = acceptor.listener.accept();
                        let (mut socket, addr) = match res {
                            Ok(socket) => socket,
                            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                                break 'accept;
                            }
                            Err(err) => {
                                (handler)(Event::Error(
                                    None,
                                    NetError::from(err).chain("Accept to peer".to_owned()),
                                ));
                                continue 'accept;
                            }
                        };

                        // Assign token
                        let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
                        let token = Token(token);

                        // Register event intreast
                        let res = poll.registry().register(
                            &mut socket,
                            token,
                            Interest::READABLE | Interest::WRITABLE,
                        );
                        if let Err(err) = res {
                            (handler)(Event::Error(
                                Some(token),
                                NetError::from(err).chain("Register accepted".to_owned()),
                            ));
                            continue 'accept;
                        }

                        let mut peer = Peer::new(socket);

                        // Should already be connected
                        // Setup the socket
                        let res = peer.connect();
                        if let Err(err) = res {
                            (handler)(Event::Error(
                                Some(token),
                                NetError::from(err).chain("Setup accepted socket".to_owned()),
                            ));
                            continue 'accept;
                        }

                        (handler)(Event::Accepted(event.token(), addr));

                        // Register peer
                        peers.insert(token, peer);
                    }
                }
            } else {
                warn!("Got event for unknown token");
            }
        }
    }
}

struct Peer<S> {
    conected: bool,

    writeable: bool,

    write_buffer: Buffer,
    read_buffer: Buffer,

    socket: S,
}

impl<S> Peer<S> {
    pub fn new(socket: S) -> Self {
        Peer {
            conected: false,
            writeable: false,
            write_buffer: Buffer::new(),
            read_buffer: Buffer::new(),
            socket,
        }
    }
}

impl Peer<TcpStream> {
    fn connect(&mut self) -> NetResult<()> {
        self.conected = true;
        self.socket.set_nodelay(true)?;

        Ok(())
    }
}

impl<S> Peer<S>
where
    for<'a> &'a mut S: Write,
{
    fn write_packet<P: Packet>(&mut self, packet: P, temp: &mut Buffer) -> NetResult<()> {
        temp.reset();

        // Write the packet to the buffer
        {
            let expected_size = HEADER_SIZE + packet.expected_size();
            let mut buffer = temp.get_unwritten(expected_size);

            let header = Header::new(&mut buffer);

            let available = buffer.len();
            let remaining = packet.write_buf(buffer);
            let packet_size = available - remaining.len();

            header
                .write(packet_size)
                .map_err(|_| NetError::OversizedPacket(packet_size))?;

            let total_written = expected_size - remaining.len();

            // Safety: We wrote something
            unsafe {
                temp.advance_write(total_written);
            }
        }

        // Write the buffer to the socket
        {
            let writeable = raw_write(&mut self.socket, temp)?;
            self.writeable = writeable;

            // Store any data not written to the socket untill the next writeable event
            self.write_buffer.copy_from(temp.get_written());
        }

        Ok(())
    }

    fn write_remaining(&mut self) -> NetResult<()> {
        let writeable = raw_write(&mut self.socket, &mut self.write_buffer)?;
        self.writeable = writeable;

        // Move any remaining data to the front of the buffer
        self.write_buffer.consume(0);

        Ok(())
    }
}

impl<S: Read> Peer<S> {
    fn read_packet<P: Packet>(&mut self, temp: &mut Buffer) -> NetResult<Option<P>> {
        temp.reset();

        // Copy any unprocessed data from last read
        temp.copy_from(self.read_buffer.get_written());
        self.read_buffer.reset();

        // A packet may be split across multiple read calls
        // And a single read call may return multiple packets
        let packet = loop {
            // Attempt to parse a packet
            {
                let mut maybe_complete_packet_buf = temp.get_written();

                // Check if a complete packet is available
                let len = Header::read(&mut maybe_complete_packet_buf);
                if let Some(len) = len {
                    let available = maybe_complete_packet_buf.len();
                    if available >= len {
                        // There is a packet available
                        // Read it
                        let mut complete_packet_buf = temp.advance_read(len);
                        let packet = P::read_buf(&mut complete_packet_buf)
                            .map_err(|err| NetError::ParsingError(err))?;

                        if complete_packet_buf.len() > 0 {
                            warn!("Packet not completely read");
                        }

                        break Some(packet);
                    }
                }
            }

            // Not enough data was available
            // Read some more for the next irreration
            let readable = raw_read_once(&mut self.socket, temp)?;
            if !readable {
                break None;
            }
        };

        // Keep unprocessed data for a future read
        self.read_buffer.copy_from(temp.get_written());

        Ok(packet)
    }
}

// Returns if the socket is still writeable
// Callees need to handle any data remaining in `buffer`
fn raw_write<S: Write>(mut socket: S, buffer: &mut Buffer) -> NetResult<bool> {
    while !buffer.is_empty() {
        let to_write = buffer.get_written();

        let res = socket.write(to_write);
        match res {
            Ok(0) => {
                // Write zero means that the connection got closed
                return Err(NetError::PeerClosed);
            }
            Ok(count) => {
                // Data has been read from the buffer and written to the socket
                // Advance the read idx so data doesn't get written multiple times
                buffer.advance_read(count);
            }

            // An error case means nothing has been written
            // Don't need to update `buffer`
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(false);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    Ok(true)
}

// Returns if the socket is still readable
fn raw_read_once<S: Read>(mut socket: S, buffer: &mut Buffer) -> NetResult<bool> {
    let read_dest = buffer.get_unwritten(PROBE_LENGTH);

    // Need loop in the unlikely case of an interruption
    loop {
        let res = socket.read(read_dest);
        match res {
            Ok(0) => {
                // Read zero means that the connection got closed
                return Err(NetError::PeerClosed);
            }
            Ok(count) => {
                // Data has been read from the socket and written to the buffer
                // Advance the write idx so data doesn't get overwritten
                // Safety: We read something
                unsafe {
                    buffer.advance_write(count);
                }
            }

            // An error case means nothing has been read
            // Don't need to update `buffer`
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                return Ok(false);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(err) => {
                return Err(err.into());
            }
        }

        return Ok(true);
    }
}

struct Acceptor<L> {
    listener: L,
}

const HEADER_SIZE: usize = 4;
struct Header<'a>(&'a mut [u8; HEADER_SIZE]);

impl<'a> Header<'a> {
    /// Needs at least `HEADER_SIZE` bytes in `buffer`
    pub fn new(buffer: &mut &'a mut [u8]) -> Self {
        // Lifetime dance taken from `impl Write for &mut [u8]`.
        let (header, remaining) = mem::take(buffer).split_array_mut();
        *buffer = remaining;

        Self(header)
    }

    /// Returns Err if len doesn't fit
    pub fn write(self, len: usize) -> Result<(), ()> {
        let header: u32 = len.try_into().map_err(|_| ())?;
        let header: [u8; HEADER_SIZE] = header.to_le_bytes();

        *self.0 = header;

        Ok(())
    }

    pub fn read(buffer: &mut &[u8]) -> Option<usize> {
        if buffer.len() < HEADER_SIZE {
            return None;
        }

        let (header, remaining) = buffer.split_array_ref();
        *buffer = remaining;

        Some(u32::from_le_bytes(*header) as _)
    }
}

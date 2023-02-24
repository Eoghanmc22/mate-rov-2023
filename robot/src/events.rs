use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender, TrySendError};
use tracing::error;

use crate::event::Event;

#[derive(Debug, Clone)]
pub struct EventHandle {
    peers: Vec<Sender<Arc<Event>>>,
    listner: Option<Receiver<Arc<Event>>>,
}

impl EventHandle {
    pub fn create(count: usize) -> Vec<EventHandle> {
        let mut peers = Vec::new();
        let mut listners = Vec::new();

        for _ in 0..count {
            let (tx, rx) = crossbeam::channel::bounded(50);
            peers.push(tx);
            listners.push(rx);
        }

        listners
            .into_iter()
            .map(|listner| EventHandle {
                peers: peers.clone(),
                listner: Some(listner),
            })
            .collect()
    }

    #[tracing::instrument]
    pub fn send(&mut self, event: Event) {
        let event = Arc::new(event);
        let mut dropped_peers = Vec::new();

        for (idx, peer) in self.peers.iter().enumerate() {
            let ret = peer.try_send(event.clone());
            if let Err(err) = ret {
                match err {
                    TrySendError::Full(_) => error!("Message channel full, event dropped"),
                    TrySendError::Disconnected(_) => {
                        dropped_peers.push(idx);
                    }
                }
            }
        }

        for idx in dropped_peers.into_iter().rev() {
            self.peers.remove(idx);
        }
    }

    pub fn listner(&self) -> Option<&Receiver<Arc<Event>>> {
        self.listner.as_ref()
    }

    pub fn take_listner(&mut self) -> Option<Receiver<Arc<Event>>> {
        self.listner.take()
    }
}

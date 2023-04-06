use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender, TrySendError};
use tracing::error;

use crate::event::Event;

/// Facilitates communication between systems
#[derive(Debug, Clone)]
pub struct EventHandle {
    peers: Vec<(usize, Sender<Arc<Event>>)>,
    listner: Option<Receiver<Arc<Event>>>,
    id: usize,
}

impl EventHandle {
    #[must_use] pub fn create(count: usize) -> Vec<Self> {
        let mut peers = Vec::new();
        let mut listners = Vec::new();

        for id in 0..count {
            let (tx, rx) = crossbeam::channel::bounded(50);
            peers.push((id, tx));
            listners.push(rx);
        }

        listners
            .into_iter()
            .enumerate()
            .map(|(id, listner)| Self {
                peers: peers.clone(),
                listner: Some(listner),
                id,
            })
            .collect()
    }

    #[tracing::instrument]
    pub fn send(&mut self, event: Event) {
        let event = Arc::new(event);
        let mut dropped_peers = Vec::new();

        for (idx, (id, peer)) in self.peers.iter().enumerate() {
            let ret = peer.try_send(event.clone());
            if let Err(err) = ret {
                match err {
                    TrySendError::Full(_) => {
                        error!("Message channel full, event dropped. Peer id: {id}")
                    }
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

    #[must_use] pub fn listner(&self) -> Option<&Receiver<Arc<Event>>> {
        self.listner.as_ref()
    }

    pub fn take_listner(&mut self) -> Option<Receiver<Arc<Event>>> {
        self.listner.take()
    }

    #[must_use] pub fn id(&self) -> usize {
        self.id
    }
}

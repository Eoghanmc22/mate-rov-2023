use std::sync::Arc;

use crossbeam::channel::{Receiver, Sender, TrySendError};
use fxhash::FxHashMap as HashMap;
use tracing::error;

use crate::{event::Event, SystemId};

/// Facilitates communication between systems
#[derive(Debug, Clone)]
pub struct EventHandle {
    peers: HashMap<SystemId, Sender<Arc<Event>>>,
    listner: Option<Receiver<Arc<Event>>>,
    id: SystemId,
}

impl EventHandle {
    #[must_use]
    pub fn create(ids: impl IntoIterator<Item = SystemId>) -> HashMap<SystemId, Self> {
        let mut peers = HashMap::default();
        let mut listners = Vec::default();

        for id in ids {
            let (tx, rx) = crossbeam::channel::bounded(50);
            let preavious = peers.insert(id, tx);
            listners.push((id, rx));

            if let Some(_) = preavious {
                panic!("Duplicate id {id:?}");
            }
        }

        listners
            .into_iter()
            .map(|(id, listner)| {
                (
                    id,
                    Self {
                        peers: peers.clone(),
                        listner: Some(listner),
                        id,
                    },
                )
            })
            .collect()
    }

    #[tracing::instrument]
    pub fn send(&mut self, event: Event) {
        let event = Arc::new(event);
        let mut dropped_peers = Vec::new();

        for (id, peer) in self.peers.iter() {
            let ret = peer.try_send(event.clone());
            if let Err(err) = ret {
                match err {
                    TrySendError::Full(_) => {
                        error!("Message channel full, event dropped. Peer id: {id:?}");
                    }
                    TrySendError::Disconnected(_) => {
                        dropped_peers.push(*id);
                    }
                }
            }
        }

        for idx in dropped_peers.into_iter().rev() {
            self.peers.remove(&idx);
        }
    }

    #[tracing::instrument]
    pub fn send_single(&mut self, event: Event, peer_id: SystemId) {
        let event = Arc::new(event);

        if let Some(peer) = self.peers.get(&peer_id) {
            let ret = peer.try_send(event.clone());
            if let Err(err) = ret {
                match err {
                    TrySendError::Full(_) => {
                        error!("Message channel full, event dropped. Peer id: {peer_id:?}");
                    }
                    TrySendError::Disconnected(_) => {
                        self.peers.remove(&peer_id);
                    }
                }
            }
        }
    }

    #[must_use]
    pub const fn listner(&self) -> Option<&Receiver<Arc<Event>>> {
        self.listner.as_ref()
    }

    pub fn take_listner(&mut self) -> Option<Receiver<Arc<Event>>> {
        self.listner.take()
    }

    #[must_use]
    pub const fn id(&self) -> SystemId {
        self.id
    }
}

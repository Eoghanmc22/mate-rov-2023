use std::{thread::Scope, time::Instant};

use common::{
    store::{tokens, Store, UpdateCallback},
    types::{Armed, RobotStatus, Speed},
};
use tracing::{span, Level};

use crate::{event::Event, events::EventHandle};

use super::{motor, System};

pub struct StatusSystem;

impl System for StatusSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listener = events.take_listner().unwrap();

        spawner.spawn(move || {
            span!(Level::INFO, "Status manager");

            let mut store = Store::new(move |update| events.send(Event::Store(update)));
            // let mut store = Store::new(move |update| {});
            let mut peers = 0;
            let mut last_status = None;

            for event in listener.into_iter() {
                let recompute_state = match &*event {
                    Event::PeerConnected(_) => {
                        peers += 1;
                        true
                    }
                    Event::PeerDisconnected(_) => {
                        peers -= 1;
                        true
                    }
                    Event::Store(update) => {
                        store.handle_update_shared(update);
                        true
                    }
                    Event::Error(_) => {
                        // TODO
                        true
                    }
                    _ => false,
                };

                if recompute_state {
                    let status = compute_status(&store, peers);

                    if last_status != Some(status) {
                        store.insert(&tokens::STATUS, status);

                        last_status = Some(status);
                    }
                }
            }
        });

        Ok(())
    }
}

fn compute_status<C: UpdateCallback>(store: &Store<C>, peers: i32) -> RobotStatus {
    if peers == 0 {
        return RobotStatus::NoPeer;
    }

    let mut state = RobotStatus::Ready;

    let now = Instant::now();
    if let Some(data) = store.get(&tokens::ARMED) {
        let (armed, time_stamp) = &*data;

        if matches!(armed, Armed::Armed) && now - *time_stamp < motor::MAX_UPDATE_AGE {
            state = RobotStatus::Armed;

            if let Some(data) = store.get(&tokens::MOTOR_SPEED) {
                let (speeds, time_stamp) = &*data;

                if now - *time_stamp < motor::MAX_UPDATE_AGE {
                    let max_speed = speeds
                        .values()
                        .map(|it| it.0.get().abs())
                        .max_by(f64::total_cmp);
                    if let Some(max_speed) = max_speed {
                        state = RobotStatus::Moving(Speed::new(max_speed));
                    }
                }
            }
        }
    }

    state
}

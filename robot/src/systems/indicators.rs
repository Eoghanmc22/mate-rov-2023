use std::{
    iter,
    thread::{self, Scope},
    time::{Duration, Instant},
    usize,
};

use common::{
    error::LogErrorExt,
    store::{tokens, Store, UpdateCallback},
    types::Armed,
};
use crossbeam::channel::{bounded, TrySendError};
use rgb::{ComponentMap, RGB8};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use tracing::{info, span, Level};

use crate::{event::Event, events::EventHandle};

use super::{motor, System};

pub struct IndicatorsSystem;

impl System for IndicatorsSystem {
    fn start<'scope>(
        mut events: EventHandle,
        spawner: &'scope Scope<'scope, '_>,
    ) -> anyhow::Result<()> {
        let listener = events.take_listner().unwrap();
        let (tx, rx) = bounded(1);

        spawner.spawn(move || {
            span!(Level::INFO, "Indicator controller");

            let mut store = Store::new(|_| {});
            let mut peers = 0;

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
                    let state = compute_state(&store, peers);
                    let rst = tx.try_send(state);
                    match rst {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {}
                        error @ Err(TrySendError::Disconnected(_)) => {
                            error.log_error("Send new led state");
                        }
                    }
                }
            }
        });

        spawner.spawn(move || {
            span!(Level::INFO, "RGB LED thread");

            let mut spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 6_000_000, Mode::Mode0)
                .expect("Open led spi");

            let interval = Duration::from_millis(10);
            let effect_length = Duration::from_millis(500);
            let steps = (effect_length.as_secs_f64() / interval.as_secs_f64()) as usize;

            let mut state = IndicatorState::NoPeer;
            let mut tick_counter = 0;
            let mut last_color = RGB8::default();

            loop {
                let next_color = state.color(tick_counter).map(|it| GAMMA8[it as usize]);

                if next_color != last_color {
                    for step in 0..steps {
                        let color = lerp_colors(last_color, next_color, step as f64 / steps as f64);

                        let data = color_to_data(color);
                        spi.write(&data).expect("Write to rgb led");

                        thread::sleep(interval);
                    }
                } else {
                    thread::sleep(effect_length);
                }

                if let Ok(new_state) = rx.try_recv() {
                    state = new_state;
                }

                tick_counter += 1;
                last_color = next_color;
            }
        });

        Ok(())
    }
}

fn compute_state<C: UpdateCallback>(store: &Store<C>, peers: i32) -> IndicatorState {
    if peers == 0 {
        return IndicatorState::NoPeer;
    }

    let mut state = IndicatorState::Ready;

    let now = Instant::now();
    if let Some(data) = store.get(&tokens::ARMED) {
        let (armed, time_stamp) = &*data;

        if matches!(armed, Armed::Armed) && now - *time_stamp < motor::MAX_UPDATE_AGE {
            state = IndicatorState::Armed;

            if let Some(data) = store.get(&tokens::MOTOR_SPEED) {
                let (speeds, time_stamp) = &*data;

                if now - *time_stamp < motor::MAX_UPDATE_AGE {
                    let max_speed = speeds
                        .values()
                        .map(|it| (it.0.get().abs() * 255.0) as u8)
                        .max();
                    if let Some(max_speed) = max_speed {
                        state = IndicatorState::Moving(max_speed);
                    }
                }
            }
        }
    }

    state
}

enum IndicatorState {
    // No peer is connected
    // Cycling blue
    NoPeer,
    // Peer is connected and robot is disarmed
    // Solid blue
    Ready,
    // Peer is connected and robot is armed
    // Solid green
    Armed,
    // The robot is moving
    // White, brightness depends on speed
    Moving(u8),
}

impl IndicatorState {
    pub fn color(&self, tick_id: usize) -> RGB8 {
        let color = match self {
            IndicatorState::NoPeer => {
                let blue = RGB8::new(0, 0, 255);
                blue * (tick_id % 5).min(1) as u8
            }
            IndicatorState::Ready => RGB8::new(255, 0, 255),
            IndicatorState::Armed => RGB8::new(0, 255, 0),
            IndicatorState::Moving(speed) => RGB8::new(*speed, *speed, *speed),
        };

        color / 3
    }
}

// From smart_led crate
const GAMMA8: [u8; 256] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5,
    5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 13, 13, 13, 14,
    14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21, 21, 22, 22, 23, 24, 24, 25, 25, 26, 27,
    27, 28, 29, 29, 30, 31, 32, 32, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41, 42, 43, 44, 45, 46,
    47, 48, 49, 50, 50, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 66, 67, 68, 69, 70, 72,
    73, 74, 75, 77, 78, 79, 81, 82, 83, 85, 86, 87, 89, 90, 92, 93, 95, 96, 98, 99, 101, 102, 104,
    105, 107, 109, 110, 112, 114, 115, 117, 119, 120, 122, 124, 126, 127, 129, 131, 133, 135, 137,
    138, 140, 142, 144, 146, 148, 150, 152, 154, 156, 158, 160, 162, 164, 167, 169, 171, 173, 175,
    177, 180, 182, 184, 186, 189, 191, 193, 196, 198, 200, 203, 205, 208, 210, 213, 215, 218, 220,
    223, 225, 228, 231, 233, 236, 239, 241, 244, 247, 249, 252, 255,
];

fn lerp_colors(from: RGB8, to: RGB8, alpha: f64) -> RGB8 {
    let r = from.r as f64 + (to.r as f64 - from.r as f64) * alpha;
    let g = from.g as f64 + (to.g as f64 - from.g as f64) * alpha;
    let b = from.b as f64 + (to.b as f64 - from.b as f64) * alpha;

    RGB8::new(r as u8, g as u8, b as u8)
}

fn color_to_data(color: RGB8) -> Vec<u8> {
    let mut data = Vec::new();

    byte_to_data(&mut data, color.g);
    byte_to_data(&mut data, color.r);
    byte_to_data(&mut data, color.b);

    data
}

fn byte_to_data(data: &mut Vec<u8>, byte: u8) {
    const LED_T0: u8 = 0b11000000;
    const LED_T1: u8 = 0b11111000;

    for bit in 0..8 {
        if byte & (0x80 >> bit) != 0 {
            data.push(LED_T1);
        } else {
            data.push(LED_T0);
        }
    }
}

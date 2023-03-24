use std::{
    thread::{self, Scope},
    time::Duration,
    usize,
};

use common::{
    error::LogErrorExt,
    store::{self, tokens},
    types::RobotStatus,
};
use crossbeam::channel::{bounded, TrySendError};
use rgb::RGB8;
use tracing::{span, Level};

use crate::{
    event::Event,
    events::EventHandle,
    peripheral::neopixel::{self, NeoPixel},
    systems::stop,
};

use super::System;

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

            for event in listener.into_iter() {
                match &*event {
                    Event::Store(update) => {
                        if let Some(status) = store::handle_update(&tokens::STATUS, update) {
                            let rst = tx.try_send(status);
                            match rst {
                                Ok(()) => {}
                                Err(TrySendError::Full(_)) => {}
                                error @ Err(TrySendError::Disconnected(_)) => {
                                    error.log_error("Send new led state");
                                }
                            }
                        }
                    }
                    Event::Exit => {
                        return;
                    }
                    _ => {}
                }
            }
        });

        spawner.spawn(move || {
            span!(Level::INFO, "RGB LED thread");

            let mut neopixel =
                NeoPixel::new(NeoPixel::SPI_BUS, NeoPixel::SPI_SELECT, NeoPixel::SPI_CLOCK)
                    .expect("Open neopixel");

            let interval = Duration::from_millis(10);
            let effect_length = Duration::from_millis(500);
            let steps = (effect_length.as_secs_f64() / interval.as_secs_f64()) as usize;

            let mut state = RobotStatus::NoPeer;
            let mut tick_counter = 0;
            let mut last_color = RGB8::default();

            while !stop::world_stopped() {
                let next_color = neopixel::correct_color(state.color(tick_counter));

                if next_color != last_color {
                    for step in 0..steps {
                        if stop::world_stopped() {
                            return;
                        }

                        let color = lerp_colors(last_color, next_color, step as f64 / steps as f64);

                        neopixel.write_color_raw(color).expect("Write to rgb led");

                        thread::sleep(interval);
                    }
                } else {
                    thread::sleep(effect_length);
                }

                if let Ok(new_state) = rx.try_recv() {
                    state = *new_state;
                }

                tick_counter += 1;
                last_color = next_color;
            }
        });

        Ok(())
    }
}

trait StatusColorExt {
    fn color(&self, tick_id: usize) -> RGB8;
}

impl StatusColorExt for RobotStatus {
    fn color(&self, tick_id: usize) -> RGB8 {
        let color = match self {
            RobotStatus::NoPeer => {
                let blue = RGB8::new(0, 0, 255);
                blue * (tick_id % 3).min(1) as u8
            }
            RobotStatus::Ready => RGB8::new(0, 0, 255),
            RobotStatus::Armed => RGB8::new(0, 255, 0),
            RobotStatus::Moving(speed) => {
                lerp_colors(RGB8::new(0, 0, 0), RGB8::new(255, 255, 255), speed.get())
            }
        };

        color / 3
    }
}

fn lerp_colors(from: RGB8, to: RGB8, alpha: f64) -> RGB8 {
    let r = from.r as f64 + (to.r as f64 - from.r as f64) * alpha;
    let g = from.g as f64 + (to.g as f64 - from.g as f64) * alpha;
    let b = from.b as f64 + (to.b as f64 - from.b as f64) * alpha;

    RGB8::new(r as u8, g as u8, b as u8)
}

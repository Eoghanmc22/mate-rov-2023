pub mod camera;

use common::types::Movement;
use egui::epaint::ahash::HashMap;
use opencv::prelude::*;

pub type Mats = HashMap<MatId, Mat>;
pub type SourceFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<()>>;
pub type ProcessorFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<Movement>>;
// pub type SinkFn = Box<dyn FnMut(Mat)>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MatId {
    RAW,
}

pub struct Source {
    pub label: String,
    pub source: SourceFn,
}

impl Source {
    pub fn new<F: FnMut(&mut Mats) -> anyhow::Result<()> + 'static>(
        label: String,
        source: F,
    ) -> Self {
        Self {
            label,
            source: Box::new(source),
        }
    }

    pub fn source(&mut self, mats: &mut Mats) -> anyhow::Result<()> {
        (self.source)(mats)
    }
}

pub struct Processor {
    pub label: String,
    pub processor: ProcessorFn,
}

impl Processor {
    pub fn new<F: FnMut(&mut Mats) -> anyhow::Result<Movement> + 'static>(
        label: String,
        processor: F,
    ) -> Self {
        Self {
            label,
            processor: Box::new(processor),
        }
    }

    pub fn process(&mut self, mats: &mut Mats) -> anyhow::Result<Movement> {
        (self.processor)(mats)
    }
}

pub struct Pipeline {
    pub source: Source,
    pub processors: Vec<Processor>,
}

impl Pipeline {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            processors: Vec::new(),
        }
    }

    pub fn execute(&mut self) -> anyhow::Result<(Mats, Movement)> {
        let mut movement = Movement::default();

        let mut images = Default::default();
        self.source.source(&mut images)?;

        for stage in &mut self.processors {
            movement += stage.process(&mut images)?;
        }

        Ok((images, movement))
    }
}

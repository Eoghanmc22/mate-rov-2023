use std::fmt::{self, Display, Formatter};

use common::types::Movement;
use egui::epaint::ahash::HashMap;
use opencv::prelude::*;

pub type Mats = HashMap<MatId, Mat>;
pub type SourceFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<bool>>;
pub type ProcessorFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<Movement>>;
pub type PipelineProto = Vec<PipelineStage>;

/// Repersents a image created in the image process pipeline
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MatId {
    Raw,
}

impl Display for MatId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Raw => "Raw",
        };

        write!(f, "{name}")
    }
}

/// Repersents a stage in the image processing pipeline
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PipelineStage {}

impl PipelineStage {
    pub fn construct(&self) -> ProcessorFn {
        todo!()
    }
}

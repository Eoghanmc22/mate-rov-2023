use common::types::Movement;
use egui::epaint::ahash::HashMap;
use opencv::prelude::*;

pub type Mats = HashMap<MatId, Mat>;
pub type SourceFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<bool>>;
pub type ProcessorFn = Box<dyn FnMut(&mut Mats) -> anyhow::Result<Movement>>;
pub type PipelineProto = Vec<PipelineStage>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MatId {
    Raw,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PipelineStage {}

impl PipelineStage {
    pub fn construct(&self) -> ProcessorFn {
        match self {
            _ => todo!(),
        }
    }
}

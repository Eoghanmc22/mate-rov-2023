//! Infrastructure to serialize and recover data

use std::{any::Any, marker::PhantomData};

use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};

pub type BackingType = Vec<u8>;

/// Repersents a type that can be serialized to and deserialized from another type
/// Usually a `BackingType`
pub trait TypeAdapter<Output> {
    fn serialize(&self, obj: &dyn Any) -> Option<Output>;
    fn deserialize(&self, data: &Output) -> Option<Box<dyn Any + Send + Sync>>;
}

pub struct Adapter<T>(PhantomData<T>);

// Current blanket impls

impl<T> TypeAdapter<BackingType> for Adapter<T>
where
    for<'a> T: Serialize + Deserialize<'a> + Any + Send + Sync,
{
    fn serialize(&self, obj: &dyn Any) -> Option<BackingType> {
        let obj = obj.downcast_ref::<T>()?;
        options().serialize(obj).ok()
    }

    fn deserialize(&self, data: &BackingType) -> Option<Box<dyn Any + Send + Sync>> {
        let obj = options().deserialize::<T>(data).ok()?;
        Some(Box::new(obj))
    }
}

impl<B> Default for Adapter<B> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// The serializeation settings used
fn options() -> impl Options {
    DefaultOptions::new()
}

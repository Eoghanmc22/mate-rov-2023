//! Infrastructure to serialize and recover data

use std::{any::Any, marker::PhantomData, time::Instant};

use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};

pub type BackingType = Vec<u8>;

/// Repersents a type that can be serialized to and deserialized from another type
/// Usually a `BackingType`
pub trait TypeAdapter<Output> {
    fn serialize(&self, obj: &dyn Any) -> Option<Output>;
    fn deserialize(&self, data: &Output) -> Option<Box<dyn Any + Send + Sync>>;
}

/// Trait ontop of `TypeAdapter` that encodes the expected type
/// Used improve type checking
pub trait TypedAdapter<Output>: TypeAdapter<Output> {
    type Data;
}

pub struct Adapter<T>(PhantomData<T>);
pub struct TimestampedAdapter<T>(PhantomData<T>);

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

impl<T, B> TypedAdapter<B> for Adapter<T>
where
    Adapter<T>: TypeAdapter<B>,
{
    type Data = T;
}

impl<B> Default for Adapter<B> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> TypeAdapter<BackingType> for TimestampedAdapter<(T, Instant)>
where
    for<'a> T: Serialize + Deserialize<'a> + Any + Send + Sync,
{
    fn serialize(&self, obj: &dyn Any) -> Option<BackingType> {
        let (obj, _) = obj.downcast_ref::<(T, Instant)>()?;
        options().serialize(obj).ok()
    }

    fn deserialize(&self, data: &BackingType) -> Option<Box<dyn Any + Send + Sync>> {
        let obj = options().deserialize::<T>(data).ok()?;
        Some(Box::new((obj, Instant::now())))
    }
}

impl<T, B> TypedAdapter<B> for TimestampedAdapter<T>
where
    TimestampedAdapter<T>: TypeAdapter<B>,
{
    type Data = T;
}

impl<B> Default for TimestampedAdapter<B> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// The serializeation settings used
fn options() -> impl Options {
    DefaultOptions::new()
}

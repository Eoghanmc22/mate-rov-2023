use std::{any::Any, marker::PhantomData, time::Instant};

use bincode::{DefaultOptions, Options};
use serde::{Deserialize, Serialize};

pub type BackingType = Vec<u8>;

pub trait TypeAdapter<Output> {
    fn serialize(&self, obj: &dyn Any) -> Option<Output>;
    fn deserialize(&self, data: &Output) -> Option<Box<dyn Any + Send + Sync>>;
}

pub trait TypedAdapter<Output>: TypeAdapter<Output> {
    type Data;
}

pub struct Adapter<T>(PhantomData<T>);

pub struct TimestampedAdapter<T>(PhantomData<T>);

// Current automatic impls

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

fn options() -> impl Options {
    DefaultOptions::new()
}

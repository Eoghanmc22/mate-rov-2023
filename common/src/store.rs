//! Implementation of a store that can store key value pairs where the value is any type
//! Generates updates to keep surface and robot in sync

pub mod adapters;
pub mod tokens;

use std::{
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use fxhash::FxHashMap as HashMap;
use tracing::error;

use crate::error::LogErrorExt;

pub type Key = KeyImpl;
pub type Value = Arc<dyn Any + Send + Sync>;
pub type Update = (Key, Option<Value>);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Token<V>(pub KeyImpl, PhantomData<V>);

impl<V> Token<V> {
    pub fn new(key: impl Into<KeyImpl>) -> Self {
        Self(key.into(), PhantomData)
    }

    pub const fn new_const(key: &'static str) -> Self {
        Self(KeyImpl::Static(key), PhantomData)
    }
}

pub struct Store<C> {
    owned: HashMap<Key, Value>,
    shared: HashMap<Key, Value>,
    timestamps: HashMap<Key, Instant>,
    callback: C,
}

impl<C: UpdateCallback> Store<C> {
    pub fn new(update_callback: C) -> Self {
        Self {
            owned: Default::default(),
            shared: Default::default(),
            timestamps: Default::default(),
            callback: update_callback,
        }
    }

    pub fn insert<V: Any + Send + Sync>(&mut self, key: &Token<V>, value: V) {
        if self.shared.contains_key(&key.0) {
            error!("Tried to update a shared key: {:?}", key.0);
            return;
        }

        let value = Arc::new(value);

        self.callback.call((key.0.clone(), Some(value.clone())));
        self.owned.insert(key.0.clone(), value);
        self.timestamps.insert(key.0.clone(), Instant::now());
    }

    pub fn remove<V: Any>(&mut self, key: &Token<V>) {
        if self.shared.contains_key(&key.0) {
            error!("Tried to remove a shared key: {:?}", key.0);
            return;
        }

        self.callback.call((key.0.clone(), None));
        self.owned.remove(&key.0);
        self.timestamps.insert(key.0.clone(), Instant::now());
    }

    pub fn refresh(&mut self) {
        for (key, data) in &self.owned {
            self.callback.call((key.clone(), Some(data.clone())))
        }
    }
}

impl<C> Store<C> {
    pub fn get<V: Any + Send + Sync>(&self, key: &Token<V>) -> Option<Arc<V>> {
        self.owned
            .get(&key.0)
            .or_else(|| self.shared.get(&key.0))
            .cloned()
            .and_then(|it| it.downcast::<V>().ok())
    }

    pub fn get_with_time<V: Any + Send + Sync>(
        &self,
        key: &Token<V>,
    ) -> Option<(Option<Arc<V>>, Instant)> {
        self.timestamps
            .get(&key.0)
            .map(|time| (self.get(key), *time))
    }

    pub fn get_alive<V: Any + Send + Sync>(
        &self,
        key: &Token<V>,
        max_age: Duration,
    ) -> Option<Arc<V>> {
        self.get_with_time(key).and_then(|(entry, timestamp)| {
            if timestamp.elapsed() < max_age {
                entry
            } else {
                None
            }
        })
    }

    pub fn is_owned<V: Any>(&self, key: &Token<V>) -> bool {
        self.is_owned_key(&key.0)
    }

    pub fn is_owned_key(&self, key: &Key) -> bool {
        self.owned.contains_key(key)
    }

    pub fn reset(&mut self) {
        self.owned.clear();
        self.shared.clear();
        self.timestamps.clear();
    }

    pub fn reset_shared(&mut self) {
        self.shared.clear();
    }

    pub fn handle_update_shared(&mut self, update: &Update) {
        if self.owned.contains_key(&update.0) {
            Err::<(), _>(format!("Bad update")).log_error("handle_update_shared");
            return;
        }

        if let Some(ref data) = update.1 {
            self.shared.insert(update.0.clone(), data.clone());
        } else {
            self.shared.remove(&update.0);
        }

        self.timestamps.insert(update.0.clone(), Instant::now());
    }

    pub fn handle_update_owned(&mut self, update: &Update) {
        if self.shared.contains_key(&update.0) {
            Err::<(), _>(format!("Bad update")).log_error("handle_update_owned");
            return;
        }

        if let Some(ref data) = update.1 {
            self.owned.insert(update.0.clone(), data.clone());
        } else {
            self.owned.remove(&update.0);
        }

        self.timestamps.insert(update.0.clone(), Instant::now());
    }
}

pub trait UpdateCallback {
    fn call(&mut self, update: Update);
}

impl<F> UpdateCallback for F
where
    F: FnMut(Update),
{
    fn call(&mut self, update: Update) {
        (self)(update)
    }
}

impl UpdateCallback for () {
    fn call(&mut self, _: Update) {}
}

#[derive(Debug, Clone, Eq)]
pub enum KeyImpl {
    Owned(String),
    Static(&'static str),
}

impl KeyImpl {
    pub fn owned(self) -> Self {
        Self::Owned(self.into())
    }

    pub fn as_str(&self) -> &str {
        match self {
            KeyImpl::Owned(value) => value.as_str(),
            KeyImpl::Static(value) => value,
        }
    }
}

impl From<String> for KeyImpl {
    fn from(value: String) -> Self {
        Self::Owned(value)
    }
}

impl From<&'static str> for KeyImpl {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<KeyImpl> for String {
    fn from(value: KeyImpl) -> Self {
        match value {
            KeyImpl::Owned(value) => value,
            KeyImpl::Static(value) => value.to_owned(),
        }
    }
}

impl ToString for KeyImpl {
    fn to_string(&self) -> String {
        self.to_owned().into()
    }
}

impl Hash for KeyImpl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            KeyImpl::Owned(value) => value.hash(state),
            KeyImpl::Static(value) => value.hash(state),
        }
    }
}

impl PartialEq for KeyImpl {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

pub fn create_update<V: Any + Send + Sync>(key: &Token<V>, value: V) -> Update {
    (key.0.clone(), Some(Arc::new(value)))
}

pub fn create_delete<V: Any + Send + Sync>(key: &Token<V>) -> Update {
    (key.0.clone(), None)
}

/// Ignores deletes
pub fn handle_update<V: Any + Send + Sync>(key: &Token<V>, update: &Update) -> Option<Arc<V>> {
    if key.0 == update.0 {
        update.1.clone().and_then(|it| it.downcast::<V>().ok())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn store_test() {
        let token_a = Token::new("a");
        let token_b = Token::new("b");

        let a = "This is a string".to_owned();
        let b = 5;

        let mut counter = 1;
        let mut store = Store::new(move |update: Update| {
            match counter {
                1 => assert_eq!(update.0, "a".into()),
                2 => assert_eq!(update.0, "b".into()),
                _ => unreachable!(),
            }

            counter += 1;
        });

        store.insert(&token_a, a.clone());
        store.insert(&token_b, b);

        assert_eq!(store.get(&token_a), Some(Arc::new(a)));
        assert_eq!(store.get(&token_b), Some(Arc::new(b)));
    }
}

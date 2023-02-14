pub mod adapters;
pub mod tokens;

use std::{any::Any, marker::PhantomData, sync::Arc};

use fxhash::FxHashMap as HashMap;

pub type Key = &'static str;
pub type Value = Arc<dyn Any + Send + Sync>;
pub type Update = (Key, Option<Value>);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Token<V>(pub Key, PhantomData<V>);

impl<V> Token<V> {
    pub const fn new(key: Key) -> Self {
        Self(key, PhantomData)
    }
}

pub struct Store {
    owned: HashMap<Key, Value>,
    shared: HashMap<Key, Value>,
    callback: Box<dyn FnMut(Update)>,
}

impl Store {
    pub fn new<F: FnMut(Update) + 'static>(update_callback: F) -> Self {
        Self {
            owned: Default::default(),
            shared: Default::default(),
            callback: Box::new(update_callback),
        }
    }

    pub fn get<V: Any + Send + Sync>(&self, key: &Token<V>) -> Option<Arc<V>> {
        self.owned
            .get(&key.0)
            .or_else(|| self.shared.get(&key.0))
            .map(|it| it.clone())
            .and_then(|it| it.downcast::<V>().ok())
    }

    pub fn insert<V: Any + Send + Sync>(&mut self, key: &Token<V>, value: V) {
        debug_assert!(!self.shared.contains_key(&key.0));

        let value = Arc::new(value);

        (self.callback)((key.0.clone(), Some(value.clone())));
        self.owned.insert(key.0.clone(), value);
    }

    // pub fn remove<V: Any>(&mut self, key: &Token<V>) {
    //     debug_assert!(!self.shared.contains_key(&key.0));
    //
    //     (self.callback)((key.0.clone(), None));
    //     self.owned.remove(key.0.clone());
    // }

    pub fn is_owned<V: Any>(&self, key: &Token<V>) -> bool {
        self.is_owned_key(&key.0)
    }

    pub fn is_owned_key(&self, key: &Key) -> bool {
        self.owned.contains_key(key)
    }

    pub fn handle_update(&mut self, update: &Update) {
        if self.owned.contains_key(&update.0) {
            return;
        }

        if let Some(ref data) = update.1 {
            self.shared.insert(update.0.clone(), data.clone());
        } else {
            self.shared.remove(&update.0);
        }
    }

    pub fn refresh(&mut self) {
        for (key, data) in &self.owned {
            (self.callback)((key.clone(), Some(data.clone())))
        }
    }

    // TODO: Implement `get_mut`
}

pub struct MutableEntry {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn it_works() {
        let token_a = Token::new("a");
        let token_b = Token::new("b");

        let a = "This is a string".to_owned();
        let b = 5;

        let mut counter = 1;
        let mut store = Store::new(move |update| {
            match counter {
                1 => assert_eq!(update.0, "a"),
                2 => assert_eq!(update.0, "b"),
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

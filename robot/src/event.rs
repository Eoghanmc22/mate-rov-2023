use std::sync::RwLock;
use crossbeam::atomic::AtomicCell;

pub struct Notify<T> {
    value: AtomicCell<T>,
    write_callbacks: RwLock<Vec<Box<dyn Fn(T) -> Action<T> + Send + Sync>>>
}

impl<T> Notify<T> {
    pub const fn new(val: T) -> Self {
        Notify {
            value: AtomicCell::new(val),
            write_callbacks: RwLock::new(Vec::new()),
        }
    }

    pub fn store(&self, val: T) {
        if let Some(val) = self.transform(val) {
            self.value.store(val);
        }
    }

    pub fn append_callback<F: Fn(T) -> Action<T> + Send+ Sync + 'static>(&self, callback: F) {
        self.write_callbacks.write().expect("write transform lock").push(Box::new(callback));
    }

    fn transform(&self, mut val: T) -> Option<T> {
        for callback in self.write_callbacks.read().expect("read transform lock").iter() {
            match (callback)(val) {
                Action::Continue(new_val) => {
                    val = new_val;
                }
                Action::Block => {
                    return None;
                }
            }
        }

        Some(val)
    }
}

impl<T: Copy> Notify<T> {
    pub fn load(&self) -> T {
        self.value.load()
    }

    pub fn swap(&self, val: T) -> T {
        if let Some(val) = self.transform(val) {
            self.value.swap(val)
        } else {
            self.value.load()
        }
    }
}

impl<T: Default> Default for Notify<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

pub enum Action<T> {
    Continue(T),
    Block
}

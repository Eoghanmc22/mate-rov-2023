use std::fmt::Debug;

use tracing::error;

pub trait LogError {
    fn log_error(self, message: &str);
}

impl<T, E: Debug> LogError for Result<T, E> {
    fn log_error(self, message: &str) {
        if let Err(err) = self {
            error!("{}: {:?}", message, err);
        }
    }
}

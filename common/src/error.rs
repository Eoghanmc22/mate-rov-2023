use std::fmt::Debug;

use tracing::error;

pub trait LogErrorExt {
    fn log_error(self, message: &str);
}

impl<T, E: Debug> LogErrorExt for Result<T, E> {
    fn log_error(self, message: &str) {
        if let Err(err) = self {
            error!("{}: {:?}", message, err);
        }
    }
}

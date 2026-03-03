//! Abstraction over the system clock.

use chrono::{DateTime, Local};

/// Abstraction over the system clock.
pub(crate) trait Clock {
    /// Return the current local date and time.
    fn now(&self) -> DateTime<Local>;
}

/// The real clock that reads from the system.
#[derive(Debug)]
pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Local> {
        Local::now()
    }
}

use std::time::{Duration, Instant};
use log::info;

pub(crate) struct TimeThrottledInfoLogger {
    interval: Duration,
    last_log_action: Instant,
}

impl TimeThrottledInfoLogger {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_log_action: Instant::now(),
        }
    }

    pub(crate) fn do_write(&mut self, message: impl Into<String>,) {
        let delta = self.last_log_action.elapsed();
        
        if delta > self.interval {
            info!("{}", message.into());
            self.last_log_action = Instant::now() - (delta - self.interval);
        }
    }
}

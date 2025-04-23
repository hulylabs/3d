use log::info;

pub(crate) struct ThrottledInfoLogger {
    interval: usize,
    counter: usize,
}

impl ThrottledInfoLogger {
    pub fn new(interval: usize) -> Self {
        assert!(interval > 0, "interval must be greater than 0");
        Self {
            interval,
            counter: 0,
        }
    }

    pub(crate) fn do_write(&mut self, message: impl Into<String>,) {
        self.counter = self.counter.wrapping_add(1);
        if self.counter % self.interval == 0 {
            info!("{}", message.into());
        }
    }
}

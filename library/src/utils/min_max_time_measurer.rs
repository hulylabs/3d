use std::time::{Duration, Instant};

pub struct MinMaxTimeMeasurer {
    min_time: Duration,
    max_time: Duration,
    last_time: Duration,

    time_mark: Instant,
}

impl MinMaxTimeMeasurer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            min_time: Duration::from_secs(u64::MAX),
            max_time: Duration::from_secs(0),
            last_time: Duration::from_secs(0),
            time_mark: Instant::now(),
        }
    }
    
    pub fn start(&mut self) {
        self.time_mark = Instant::now();
    }
    
    pub fn stop(&mut self) {
        let delta = self.time_mark.elapsed();
        self.min_time = self.min_time.min(delta);
        self.max_time = self.max_time.max(delta);
        self.last_time = delta;
    }

    #[must_use] 
    pub fn min_time(&self) -> Duration {
        self.min_time
    }

    #[must_use]
    pub fn max_time(&self) -> Duration {
        self.max_time
    }

    #[must_use]
    pub fn last_time(&self) -> Duration {
        self.last_time
    }
}

impl Default for MinMaxTimeMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_single_start_stop() {
        let mut system_under_test = MinMaxTimeMeasurer::new();
        let duration_to_measure = Duration::from_millis(5);
        
        system_under_test.start();
        std::thread::sleep(duration_to_measure);
        system_under_test.stop();

        assert!(system_under_test.min_time() >= duration_to_measure);
        assert!(system_under_test.max_time() >= duration_to_measure);
        assert!(system_under_test.last_time() >= duration_to_measure);
        assert!(system_under_test.min_time() <= system_under_test.max_time());
    }

    #[test]
    fn test_two_start_stop_sequences() {
        let mut system_under_test = MinMaxTimeMeasurer::new();
        let small_duration_to_measure = Duration::from_millis(5);
        let long_duration_to_measure = Duration::from_millis(25);

        system_under_test.start();
        std::thread::sleep(small_duration_to_measure);
        system_under_test.stop();
        
        assert!(system_under_test.last_time() >= small_duration_to_measure);
        
        system_under_test.start();
        std::thread::sleep(long_duration_to_measure);
        system_under_test.stop();

        assert!(system_under_test.last_time() >= long_duration_to_measure);
        
        assert!(system_under_test.min_time() >= small_duration_to_measure);
        assert!(system_under_test.max_time() >= long_duration_to_measure);
        assert!(system_under_test.min_time() <= system_under_test.max_time());
    }
}

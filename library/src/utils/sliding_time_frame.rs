use std::time::{Duration, Instant};

pub(crate) struct SlidingTimeFrame {
    millisecond_deltas: Vec<u128>,
    circular_buffer_pointer: usize,
    total_delta: u128,
    
    last_time_mark: Instant,
}

impl SlidingTimeFrame {
    #[must_use]
    pub(crate) fn new(measurements_count: usize) -> Self {
        assert!(measurements_count > 0);
        Self { 
            millisecond_deltas: vec![0; measurements_count], 
            circular_buffer_pointer: 0, 
            total_delta: 0,
            last_time_mark: Instant::now(),
        }
    }
    
    pub(crate) fn add_delta(&mut self, delta: u128) {
        let erased_value = self.millisecond_deltas[self.circular_buffer_pointer];
        self.millisecond_deltas[self.circular_buffer_pointer] = delta;
        self.total_delta += delta;
        self.total_delta -= erased_value;
        self.circular_buffer_pointer = (self.circular_buffer_pointer + 1) % self.millisecond_deltas.len();
    }
    
    pub(crate) fn sample(&mut self) {
        let current_time = Instant::now();
        let delta = current_time.duration_since(self.last_time_mark);
        self.add_delta(delta.as_millis());
        self.last_time_mark = current_time;
    }
    
    pub(crate) fn start(&mut self) {
        self.last_time_mark = Instant::now();
    }
    
    #[must_use]
    pub(crate) fn average_delta(&self) -> Duration {
        Duration::from_millis((self.total_delta as f64 / (self.millisecond_deltas.len() as f64)) as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;
    use super::*;

    #[test]
    fn test_construction() {
        let system_under_test = SlidingTimeFrame::new(5);
        assert_eq!(system_under_test.average_delta(), Duration::from_millis(0));
    }

    #[test]
    #[should_panic]
    fn test_zero_measurements_should_panic() {
        _ = SlidingTimeFrame::new(0);
    }

    #[test]
    fn test_add_delta() {
        let measurements_count = 3;
        let mut system_under_test = SlidingTimeFrame::new(measurements_count);

        let first_delta = 100;
        system_under_test.add_delta(first_delta);
        assert_eq!(system_under_test.average_delta(), Duration::from_millis((first_delta as f64 / measurements_count as f64) as u64));

        let second_delta = 200;
        system_under_test.add_delta(second_delta);
        assert_eq!(system_under_test.average_delta(), Duration::from_millis(((second_delta + first_delta) as f64 / measurements_count as f64) as u64));

        let third_delta = 300;
        system_under_test.add_delta(third_delta);
        assert_eq!(system_under_test.average_delta(), Duration::from_millis(((second_delta + first_delta + third_delta) as f64 / measurements_count as f64) as u64));

        let fourth_delta = 600;
        system_under_test.add_delta(fourth_delta);
        assert_eq!(system_under_test.average_delta(), Duration::from_millis(((second_delta + third_delta + fourth_delta) as f64 / measurements_count as f64) as u64));
    }

    #[test]
    fn test_sample_after_start() {
        let mut system_under_test = SlidingTimeFrame::new(1);

        let sleep_time = Duration::from_millis(10);
        
        system_under_test.start();
        sleep(sleep_time);
        system_under_test.sample();
        
        let actual_average = system_under_test.average_delta();
        assert!(actual_average >= sleep_time);
    }

    #[test]
    fn test_sample_without_start() {
        let mut system_under_test = SlidingTimeFrame::new(1);

        let sleep_time = Duration::from_millis(27);
        sleep(sleep_time);
        system_under_test.sample();

        let actual_average = system_under_test.average_delta();
        assert!(actual_average >= sleep_time);
    }
}
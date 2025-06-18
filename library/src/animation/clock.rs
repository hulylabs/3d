use crate::animation::clock_animation_act::{ClockAnimationAct, EndActionKind, PhaseAlive, WrapKind};
use std::time::{Duration, Instant};

pub(super) struct Clock {
    parameters: ClockAnimationAct<PhaseAlive>,
    global_clock_start: Instant,
}

impl Clock {
    #[must_use]
    pub(super) fn new(current_time: Instant, parameters: ClockAnimationAct<PhaseAlive>) -> Self {
        Self {
            parameters,
            global_clock_start: current_time,
        }
    }
    
    #[must_use]
    pub(super) fn ticking(&self, global_time: Instant) -> bool {
        self.parameters.get_time_to_live()
            .is_none_or( 
                |time_to_live| global_time.duration_since(self.global_clock_start) < time_to_live.span())
    }
    
    #[must_use]
    pub(super) fn local_time(&self, global_time: Instant) -> f64 {
        let local_forward_time = self.local_forward_time(global_time);

        if let Some(ttl) = self.parameters.get_time_to_live() {
            if ttl.reverse() {
                return (ttl.span() + self.parameters.get_birth_time_offset()).as_secs_f64() - local_forward_time;
            }
        }

        local_forward_time
    }

    #[must_use]
    fn local_forward_time(&self, global_time: Instant) -> f64 {
        let global_elapsed = global_time.duration_since(self.global_clock_start);

        if let Some(time_to_live) = self.parameters.get_time_to_live() {
            if global_elapsed > time_to_live.span() {
                return match self.parameters.get_end_action() {
                    EndActionKind::TeleportToZero => 0.0,
                    EndActionKind::LeaveAsIs => self.evaluate_time_point(time_to_live.span()),
                    EndActionKind::TeleportToEnd => time_to_live.span().as_secs_f64() * self.parameters.get_playback_speed_multiplier(),
                };
            }
        }

        self.evaluate_time_point(global_elapsed)
    }

    #[must_use]
    fn evaluate_time_point(&self, global_elapsed: Duration) -> f64 {
        let local_tile_offset = self.parameters.get_birth_time_offset().as_secs_f64();
        let local_time_multiplier = self.parameters.get_playback_speed_multiplier();
        let local_elapsed = local_tile_offset + global_elapsed.as_secs_f64() * local_time_multiplier;

        if let Some(periodization) = self.parameters.get_periodization() {
            let period = periodization.period().as_secs_f64();
            let period_count: i64 = (local_elapsed / period) as i64;
            let local_period_rest = local_elapsed - (period_count as f64) * period;
            match periodization.wrap_kind() {
                WrapKind::Restart => local_period_rest,
                WrapKind::Reverse => {
                    if 0 == period_count % 2 {
                        local_period_rest
                    } else {
                        period - local_period_rest
                    }
                }
            }
        } else {
            local_elapsed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::clock_animation_act::{ClockAnimationAct, EndActionKind, Periodization, PhaseConstruction, TimeDirection};
    use rstest::rstest;
    use std::time::Duration;

    #[must_use]
    fn infinite_animation_run() -> ClockAnimationAct<PhaseAlive> {
        ClockAnimationAct::default()
    }

    #[must_use]
    fn infinite_animation_blank() -> ClockAnimationAct<PhaseConstruction> {
        ClockAnimationAct::default()
    }

    #[test]
    fn test_ticking() {
        let start = Instant::now();
        let animation_duration = Duration::from_millis(1);
        let act 
            = ClockAnimationAct::new()
                .with_global_finite_time_to_live(animation_duration, TimeDirection::Forward)
                .make();
        let system_under_test = Clock::new(start, act);

        assert!(system_under_test.ticking(start));
        
        std::thread::sleep(animation_duration);
        assert_eq!(system_under_test.ticking(Instant::now()), false);
    }

    #[test]
    fn test_local_time_basic_forward() {
        let start = Instant::now();
        let system_under_test = Clock::new(start, infinite_animation_run());
    
        let elapsed_time = Duration::from_secs(5);
        let test_time = start + elapsed_time;
        let actual_local_time = system_under_test.local_time(test_time);
    
        assert_eq!(actual_local_time, elapsed_time.as_secs_f64());
    }
    
    #[test]
    fn test_local_time_with_birth_offset() {
        let start = Instant::now();
        let birth_offset = Duration::from_secs(3);
        let elapsed_time = Duration::from_secs(5);
    
        let animation = infinite_animation_blank()
            .birth_time_offset(birth_offset)
            .make();
        let system_under_test = Clock::new(start, animation);
    
        let actual_local_time = system_under_test.local_time(start + elapsed_time);
        let expected_local_time = (birth_offset + elapsed_time).as_secs_f64();
    
        assert_eq!(actual_local_time, expected_local_time);
    }
    
    #[test]
    fn test_local_time_with_speed_multiplier() {
        let start = Instant::now();
        let local_time_multiplier = 2.0;
        let elapsed = Duration::from_secs(5);
    
        let animation = infinite_animation_blank()
            .playback_speed_multiplier(local_time_multiplier)
            .make();
        let system_under_test = Clock::new(start, animation);
    
        let result = system_under_test.local_time(start + elapsed);
    
        assert_eq!(result, elapsed.as_secs_f64() * local_time_multiplier);
    }
    
    #[test]
    fn test_local_time_with_ttl_forward_within_span() {
        let start = Instant::now();
        let life_span = Duration::from_secs(10);
        let elapsed = life_span / 2;
    
        let animation = ClockAnimationAct::new()
            .with_global_finite_time_to_live(life_span, TimeDirection::Forward)
            .make();
        let system_under_test = Clock::new(start, animation);
    
        let actual_local_time = system_under_test.local_time(start + elapsed);
    
        assert_eq!(actual_local_time, elapsed.as_secs_f64());
    }
    
    #[test]
    fn test_local_time_with_ttl_backward_within_span() {
        let start = Instant::now();
        let life_span = Duration::from_secs(10);
        let elapsed_time = Duration::from_secs(3);
        let time_offset = Duration::from_secs(1);
    
        let animation = ClockAnimationAct::new()
            .with_global_finite_time_to_live(life_span, TimeDirection::Backward)
            .birth_time_offset(time_offset)
            .make();
        let system_under_test = Clock::new(start, animation);
    
        let actual_local_time = system_under_test.local_time(start + elapsed_time);
    
        assert_eq!(actual_local_time, (life_span - elapsed_time).as_secs_f64());
    }

    #[rstest]
    #[case(EndActionKind::TeleportToEnd , TimeDirection::Forward , 5.0)]
    #[case(EndActionKind::TeleportToEnd , TimeDirection::Backward, 3.0)]
    #[case(EndActionKind::TeleportToZero, TimeDirection::Forward , 0.0)]
    #[case(EndActionKind::TeleportToZero, TimeDirection::Backward, 8.0)]
    #[case(EndActionKind::LeaveAsIs     , TimeDirection::Forward , 8.0)]
    #[case(EndActionKind::LeaveAsIs     , TimeDirection::Backward, 0.0)]
    fn test_local_time_with_ttl_exceeded(#[case] end_action: EndActionKind, #[case] time_direction: TimeDirection, #[case] expected_local_time: f64) {
        let start = Instant::now();
        let life_span = Duration::from_secs(5);

        let animation = ClockAnimationAct::new()
            .with_global_finite_time_to_live(life_span, time_direction)
            .birth_time_offset(Duration::from_secs(3))
            .end_action(end_action)
            .make();
        let system_under_test = Clock::new(start, animation);

        let actual_local_time = system_under_test.local_time(start + life_span + Duration::from_nanos(7));

        assert_eq!(actual_local_time, expected_local_time);
    }
    
    #[test]
    fn test_local_time_periodization_restart_single_period() {
        let start = Instant::now();
        let period = Duration::from_secs(3);
        let periodization = Periodization::new(WrapKind::Restart, period);
        
        let animation = ClockAnimationAct::new()
            .periodization(Some(periodization))
            .make();
        let system_under_test = Clock::new(start, animation);
    
        // within first period
        assert_eq!(system_under_test.local_time(start + period - Duration::from_secs(1)), 2.0);
    
        // exactly at period boundary
        assert_eq!(system_under_test.local_time(start + period), 0.0);
    
        // into second period
        assert_eq!(system_under_test.local_time(start + period + Duration::from_secs(2)), 2.0);
    }
    
    #[test]
    fn test_local_time_periodization_restart_multiple_periods() {
        let start = Instant::now();
        let periodization = Periodization::new(WrapKind::Restart, Duration::from_secs(2));
        
        let animation = ClockAnimationAct::new()
            .periodization(Some(periodization))
            .make();
        let system_under_test = Clock::new(start, animation);
        
    
        // 7 / 2 = 3.5, so 3rd period, 1 second in
        let actual_local_time = system_under_test.local_time(start + Duration::from_secs(7));
        
        assert_eq!(actual_local_time, 1.0);
    }
    
    #[test]
    fn test_local_time_periodization_reverse() {
        let start = Instant::now();
        let periodization = Periodization::new(WrapKind::Reverse, Duration::from_secs(4));
        
        let animation = ClockAnimationAct::new()
            .periodization(Some(periodization))
            .make();
        let system_under_test = Clock::new(start, animation);
    
        // first period (period count = 0, even)
        let test_time = start + Duration::from_secs(2);
        assert_eq!(system_under_test.local_time(test_time), 2.0);
    
        // second period (period count = 1, odd)
        let test_time = start + Duration::from_secs(5);
        assert_eq!(system_under_test.local_time(test_time), 3.0);
    
        // third period (period count = 2, even)
        let test_time = start + Duration::from_secs(11);
        assert_eq!(system_under_test.local_time(test_time), 3.0);
    }
    
    #[test]
    fn test_local_time_periodization_with_birth_offset() {
        let start = Instant::now();
        let period = Duration::from_secs(3);
        let periodization = Periodization::new(WrapKind::Restart, period);
        let offset = Duration::from_secs(1);
        
        let animation = ClockAnimationAct::new()
            .birth_time_offset(offset)
            .periodization(Some(periodization))
            .make();
        let system_under_test = Clock::new(start, animation);
        
        let test_time = start + period - offset;
        let actual_local_time = system_under_test.local_time(test_time);
        
        assert_eq!(actual_local_time, 0.0);
    }
    
    #[test]
    fn test_local_time_periodization_with_speed_multiplier() {
        let start = Instant::now();
        let periodization = Periodization::new(WrapKind::Restart, Duration::from_secs(4));
        
        let animation = ClockAnimationAct::new()
            .playback_speed_multiplier(2.0)
            .periodization(Some(periodization))
            .make();
        let system_under_test = Clock::new(start, animation);
    
        // 1 second elapsed * 2.0 speed = 2 seconds local time
        let test_time = start + Duration::from_secs(1);
        assert_eq!(system_under_test.local_time(test_time), 2.0);
    
        // 2 seconds elapsed * 2.0 speed = 4 seconds local time = exactly one period
        let test_time = start + Duration::from_secs(2);
        assert_eq!(system_under_test.local_time(test_time), 0.0);
    }
    
    #[test]
    fn test_local_time_complex_scenario_all_features() {
        let start_time = Instant::now();
        
        let periodization = Periodization::new(WrapKind::Reverse, Duration::from_secs(6));
        let animation = ClockAnimationAct::new()
            .birth_time_offset(Duration::from_secs(1))
            .playback_speed_multiplier(1.5)
            .periodization(Some(periodization))
            .with_global_finite_time_to_live(Duration::from_secs(20), TimeDirection::Forward)
            .end_action(EndActionKind::LeaveAsIs)
            .make();
        let system_under_test = Clock::new(start_time, animation);
        
        
        let test_time = start_time + Duration::from_secs(4);
        let actual_local_time = system_under_test.local_time(test_time);
        assert_eq!(actual_local_time, 5.0);
    }
    
    #[test]
    fn test_local_time_ttl_with_periodization_exceeded() {
        let start = Instant::now();
        
        let periodization = Periodization::new(WrapKind::Restart, Duration::from_secs(3));
        let animation = ClockAnimationAct::new()
            .periodization(Some(periodization))
            .with_global_finite_time_to_live(Duration::from_secs(5), TimeDirection::Forward)
            .end_action(EndActionKind::TeleportToZero)
            .make();
        let system_under_test = Clock::new(start, animation);
        
        let test_time = start + Duration::from_secs(10);
        let actual_local_time = system_under_test.local_time(test_time);
        
        assert_eq!(actual_local_time, 0.0);
    }
    
    #[test]
    fn test_local_time_fractional_seconds_precision() {
        let start = Instant::now();
        let system_under_test = Clock::new(start, infinite_animation_run());

        let elapsed = Duration::from_millis(1500);
        let result = system_under_test.local_time(start + elapsed);
        
        assert!((result - elapsed.as_secs_f64()).abs() < f64::EPSILON);
    }
}

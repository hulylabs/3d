use crate::animation::clock::Clock;
use crate::animation::clock_animation_act::{ClockAnimationAct, PhaseAlive};
use crate::objects::common_properties::ObjectUid;
use std::collections::HashMap;
use std::time::Instant;

pub(crate) struct Animator {
    animations: HashMap<ObjectUid, Clock>,
    current_time: Instant,
}

impl Animator {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            animations: HashMap::new(),
            current_time: Instant::now(),
        }
    }

    pub(crate) fn remove_finished(&mut self) {
        self.animations.retain(|_, clock| {
            clock.ticking(self.current_time)
        })
    }

    pub(crate) fn take_time(&mut self) {
        self.current_time = Instant::now();
    }

    pub(crate) fn animate_time(&mut self, target: ObjectUid, parameters: ClockAnimationAct<PhaseAlive>) {
        self.animations.insert(target, Clock::new(self.current_time, parameters));
    }

    pub(crate) fn stop(&mut self, target: ObjectUid) {
        self.animations.remove(&target);
    }

    pub(crate) fn clear(&mut self) {
        self.animations.clear();
    }
    
    #[must_use]
    pub(crate) fn local_time_of(&self, target: ObjectUid) -> Option<f64> {
        self.animations.get(&target).map(|animation| animation.local_time(self.current_time))
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use super::*;
    use crate::animation::clock_animation_act::{ClockAnimationAct, TimeDirection};
    use crate::objects::common_properties::ObjectUid;
    use std::time::Duration;
    use more_asserts::assert_gt;

    #[test]
    fn test_empty_animator() {
        let mut system_under_test = Animator::new();

        system_under_test.take_time();
        system_under_test.remove_finished();
        system_under_test.clear();
        
        assert_eq!(system_under_test.local_time_of(ObjectUid(0)), None);
    }

    #[test]
    fn test_animate_time() {
        let mut system_under_test = Animator::new();
        let target_uid = ObjectUid(17);

        system_under_test.animate_time(target_uid, infinite_animation());

        assert!(system_under_test.local_time_of(target_uid).is_some());
    }

    #[test]
    fn test_animate_same_object_twice() {
        let mut system_under_test = Animator::new();
        let target_uid = ObjectUid(17);
        
        system_under_test.animate_time(target_uid, infinite_animation());
        let time_sample_first = system_under_test.local_time_of(target_uid);
        assert!(time_sample_first.is_some());

        system_under_test.animate_time(target_uid, infinite_animation());
        let time_sample_second = system_under_test.local_time_of(target_uid);
        assert!(time_sample_second.is_some());
    }

    #[test]
    fn test_stop_existing() {
        let mut system_under_test = Animator::new();
        let target_uid = ObjectUid(17);

        system_under_test.animate_time(target_uid, infinite_animation());
        assert!(system_under_test.local_time_of(target_uid).is_some());

        system_under_test.stop(target_uid);
        assert!(system_under_test.local_time_of(target_uid).is_none());
    }

    #[test]
    fn test_stop_nonexistent() {
        let mut system_under_test = Animator::new();
        let target_uid = ObjectUid(17);
        
        system_under_test.stop(target_uid);
        
        assert!(system_under_test.local_time_of(target_uid).is_none());
    }

    #[test]
    fn test_clear() {
        let mut system_under_test = Animator::new();
        let target_one = ObjectUid(17);
        let target_two = ObjectUid(31);

        system_under_test.animate_time(target_one, infinite_animation());
        system_under_test.animate_time(target_two, infinite_animation());
        system_under_test.clear();
        
        assert!(system_under_test.local_time_of(target_one).is_none());
        assert!(system_under_test.local_time_of(target_two).is_none());
    }
    
    #[test]
    fn test_take_time() {
        let mut system_under_test = Animator::new();
        let target_uid = ObjectUid(17);

        system_under_test.animate_time(target_uid, infinite_animation());
        let sample_one = system_under_test.local_time_of(target_uid);
        thread::sleep(Duration::from_millis(1));
        let sample_two = system_under_test.local_time_of(target_uid);

        thread::sleep(Duration::from_millis(1));
        system_under_test.take_time();
        let sample_after_time_taken = system_under_test.local_time_of(target_uid);

        assert_eq!(sample_one, sample_two, "time samples should not change without 'take_time' being called");
        assert_gt!(sample_after_time_taken.unwrap(), sample_one.unwrap(), "'time taken' did not affect time samples");
    }

    #[test]
    fn test_clean_finished_behavior() {
        let mut system_under_test = Animator::new();
        let to_be_finished = ObjectUid(17);
        let to_be_continued = ObjectUid(13);
        let animation_duration = Duration::from_nanos(1);
        let animation_act 
            = ClockAnimationAct::new()
                .with_global_finite_time_to_live(animation_duration, TimeDirection::Forward)
                .make();

        system_under_test.animate_time(to_be_finished, animation_act);
        system_under_test.animate_time(to_be_continued, infinite_animation());
        thread::sleep(animation_duration + Duration::from_millis(1));
        system_under_test.take_time();
        system_under_test.remove_finished();

        let time_after_clean = system_under_test.local_time_of(to_be_finished);
        assert!(time_after_clean.is_none(), "animation did not finished as expected");
        assert!(system_under_test.local_time_of(to_be_continued).is_some(), "infinite animation has been removed");
    }

    #[must_use]
    fn infinite_animation() -> ClockAnimationAct<PhaseAlive> {
        ClockAnimationAct::default()
    }
}
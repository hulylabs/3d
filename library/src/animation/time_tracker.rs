use crate::animation::animator::Animator;
use crate::animation::clock_animation_act::{ClockAnimationAct, PhaseAlive};
use crate::utils::object_uid::ObjectUid;
use crate::utils::version::Version;
use more_asserts::assert_ge;
use std::collections::HashMap;

pub struct TimeTracker {
    animator: Animator,
    tracked: HashMap<ObjectUid, Animatable>,
    version: Version,
}

impl TimeTracker {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self { animator: Animator::new(), tracked: HashMap::new(), version:Version(0) }
    }
    
    pub(crate) fn update_time(&mut self) {
        self.animator.take_time();
        let mut any_updated = false;
        for (uid, animatable) in self.tracked.iter_mut() {
            let new_time = self.animator.local_time_of(*uid);
            any_updated |= animatable.update_time(new_time);
        }
        if any_updated {
            self.version += 1;
        }
        self.animator.remove_finished();
    }

    pub fn launch(&mut self, target: ObjectUid, parameters: ClockAnimationAct<PhaseAlive>) {
        assert!(self.tracked.contains_key(&target));
        self.animator.animate_time(target, parameters);
    }
    
    pub fn stop(&mut self, target: ObjectUid) {
        assert!(self.tracked.contains_key(&target));
        self.animator.stop(target);
    }
    
    #[must_use]
    pub fn animating(&self, target: ObjectUid) -> bool {
        self.animator.local_time_of(target).is_some()
    }
    
    pub(crate) fn track(&mut self, target: ObjectUid, new_order: &[ObjectUid]) {
        assert_eq!(new_order.len(), self.tracked.len()+1);
        
        self.tracked.insert(target, Animatable::new(0));
        self.update_indices(new_order);
        
        self.version += 1;
    }
    
    pub(crate) fn forget(&mut self, target: ObjectUid, new_order: &[ObjectUid]) {
        if self.tracked.remove(&target).is_some() {
            assert_eq!(new_order.len(), self.tracked.len());
            self.animator.stop(target);
            self.update_indices(new_order);
            self.version += 1;
        } else {
            assert_eq!(new_order.len(), self.tracked.len());
            self.update_indices(new_order);
        }
    }
    
    pub(crate) fn clear(&mut self) {
        if self.tracked.is_empty() {
            return;
        }
        self.tracked.clear();
        self.animator.clear();
        self.version += 1;
    }
    
    pub(crate) fn write_times(&self, target: &mut [f32]) {
        assert_ge!(target.len(), self.tracked.len());
        for animatable in self.tracked.values() {
            target[animatable.index()] = animatable.time();
        }
    }

    fn update_indices(&mut self, new_order: &[ObjectUid]) {
        for (index, uid) in new_order.iter().enumerate() {
            if let Some(animatable)  = self.tracked.get_mut(uid) {
                animatable.set_index(index);
            } else {
                panic!("unknown object uid {uid}");
            } 
        }
    }

    #[must_use]
    pub(crate) fn version(&self) -> Version {
        self.version
    }
    
    #[must_use]
    pub(crate) fn tracked_count(&self) -> usize {
        self.tracked.len()
    }
}

struct Animatable {
    time: f64,
    index: usize,
}

impl Animatable {
    #[must_use]
    fn new(index: usize) -> Self {
        Self { time: 0.0, index }
    }
    
    #[must_use]
    fn time(&self) -> f32 {
        self.time as f32
    }
    
    #[must_use]
    fn update_time(&mut self, new_time: Option<f64>) -> bool {
        new_time.is_some_and(|new| { self.time = new; true })
    }
    
    fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    #[must_use]
    fn index(&self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::clock_animation_act::{ClockAnimationAct, EndActionKind, TimeDirection};
    use crate::utils::object_uid::ObjectUid;
    use crate::utils::tests::assert_utils::tests::assert_all_unique;
    use more_asserts::assert_gt;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_empty_time_tracker() {
        let mut system_under_test = TimeTracker::new();

        let version_before = system_under_test.version();
        system_under_test.update_time();
        system_under_test.clear();
        system_under_test.write_times(&mut [0_f32; 0]);
        let version_after = system_under_test.version();

        assert_eq!(version_before, version_after);
        assert_eq!(system_under_test.tracked_count(), 0);
    }

    #[test]
    fn test_track_single_object() {
        let mut system_under_test = TimeTracker::new();

        let version_before = system_under_test.version();

        let uid = ObjectUid(0);
        let order = vec![uid];
        system_under_test.track(uid, &order);

        let version_after_track = system_under_test.version();

        let mut times = vec![7.0f32; 1];
        system_under_test.write_times(&mut times);

        let version_after_write = system_under_test.version();

        assert_eq!(system_under_test.tracked_count(), 1);
        assert_eq!(times, [0.0_f32]);
        assert_ne!(version_before, version_after_track);
        assert_eq!(version_after_track, version_after_write);
    }

    #[test]
    fn test_track_multiple_objects() {
        let mut system_under_test = TimeTracker::new();
        let uids = [ObjectUid(0), ObjectUid(1), ObjectUid(2)];

        let mut versions: Vec<Version> = Vec::new();
        versions.push(system_under_test.version());

        for i in 0..uids.len() {
            let count_after_add = i + 1;
            system_under_test.track(uids[i], &uids[..count_after_add]);
            versions.push(system_under_test.version());
            assert_eq!(system_under_test.tracked_count(), count_after_add);
        }

        let mut times = vec![-3.0f32; uids.len()];
        system_under_test.write_times(&mut times);

        assert_eq!(times, vec![0.0_f32; uids.len()]);
        assert_all_unique(&mut versions);
    }

    #[test]
    fn test_stop() {
        let mut system_under_test = TimeTracker::new();
        let to_continue = ObjectUid(7);
        let to_stop = ObjectUid(5);

        system_under_test.track(to_continue, &[to_continue]);
        system_under_test.track(to_stop, &[to_continue, to_stop]);

        system_under_test.stop(to_continue);
        system_under_test.stop(to_stop);

        system_under_test.launch(to_continue, ClockAnimationAct::default());
        system_under_test.launch(to_stop, ClockAnimationAct::default());

        system_under_test.stop(to_stop);
        
        system_under_test.update_time();
        assert!(system_under_test.animating(to_continue));
        assert_eq!(system_under_test.animating(to_stop), false);

        let mut times = vec![-5.0f32; 2];
        system_under_test.write_times(&mut times);
        assert_gt!(times[0], 0.0_f32);   
        assert_eq!(times[1], 0.0_f32);   
    }
    
    #[test]
    fn test_forget_object() {
        let mut system_under_test = TimeTracker::new();
        let to_keep = ObjectUid(7);
        let to_forget = ObjectUid(5);

        system_under_test.track(to_keep, &[to_keep]);
        system_under_test.track(to_forget, &[to_keep, to_forget]);

        let version_before = system_under_test.version();
        system_under_test.forget(to_keep, &[to_forget]);
        let version_after = system_under_test.version();

        assert_ne!(version_before, version_after);
        assert_eq!(system_under_test.tracked_count(), 1);

        let mut times = vec![-5.0f32; 1];
        system_under_test.write_times(&mut times);
        assert_eq!(times, [0.0_f32]);
    }

    #[test]
    fn test_clear() {
        let mut system_under_test = TimeTracker::new();
        let first = ObjectUid(0);
        let second = ObjectUid(1);

        system_under_test.track(first, &[first]);
        system_under_test.track(second, &[first, second]);

        let version_before = system_under_test.version();
        system_under_test.clear();
        let version_after = system_under_test.version();

        let mut times = vec![0.0_f32; 0];
        system_under_test.write_times(&mut times);
        assert_ne!(version_before, version_after);
        assert_eq!(system_under_test.tracked_count(), 0);
    }

    #[test]
    fn test_launch_animation() {
        let mut system_under_test = TimeTracker::new();
        let animated = ObjectUid(13);
        let animation = ClockAnimationAct::default();

        system_under_test.track(animated, &[animated]);
        let version_before_launch = system_under_test.version();
        system_under_test.launch(animated, animation);
        let still = ObjectUid(17);
        let version_after_launch = system_under_test.version();
        system_under_test.track(still, &[animated, still]);
        
        let mut times = vec![-5.0f32; system_under_test.tracked_count()];
        system_under_test.write_times(&mut times);
        assert_eq!(times, vec![0.0_f32; system_under_test.tracked_count()]);

        system_under_test.update_time();
        let version_after_time_update = system_under_test.version();
        system_under_test.write_times(&mut times);
        assert_gt!(times[0], 0.0_f32);
        assert_eq!(times[1], 0.0_f32);
        
        assert_eq!(version_before_launch, version_after_launch);
        assert_ne!(version_after_launch, version_after_time_update);
    }
    
    #[test]
    fn test_launch_already_animated() {
        let mut system_under_test = TimeTracker::new();
        let uid = ObjectUid(13);
        let infinite_animation = ClockAnimationAct::default();
        let expected_duration = Duration::from_millis(1);
        let one_ms_animation = finite_animation(expected_duration);

        system_under_test.track(uid, &[uid]);
        system_under_test.launch(uid, infinite_animation);
        system_under_test.launch(uid, one_ms_animation);
        thread::sleep(expected_duration + Duration::from_millis(3));
        system_under_test.update_time();

        let mut times = vec![-5.0f32; system_under_test.tracked_count()];
        system_under_test.write_times(&mut times);
        assert_eq!(times, vec![0.001_f32; system_under_test.tracked_count()]);
    }

    #[test]
    fn test_write_to_buffer_order() {
        let mut system_under_test = TimeTracker::new();
        let tiny_uid = ObjectUid(13);
        let huge_uid = ObjectUid(31);
        
        system_under_test.track(tiny_uid, &[tiny_uid]);
        system_under_test.track(huge_uid, &[huge_uid, tiny_uid]);

        let tiny_uid_time = Duration::from_micros(3);
        system_under_test.launch(tiny_uid, finite_animation(tiny_uid_time));
        let huge_uid_time = Duration::from_micros(5);
        system_under_test.launch(huge_uid, finite_animation(huge_uid_time));

        thread::sleep(huge_uid_time);
        system_under_test.update_time();

        let excess_slot_marker: f32 = -99.0_f32;
        let mut buffer = vec![excess_slot_marker; 3];
        system_under_test.write_times(&mut buffer);
    
        assert_eq!(buffer, [huge_uid_time.as_secs_f32(), tiny_uid_time.as_secs_f32(), excess_slot_marker]);
    }
    
    #[must_use]
    fn finite_animation(expected_duration: Duration) -> ClockAnimationAct<PhaseAlive> {
        ClockAnimationAct::new()
            .with_global_finite_time_to_live(expected_duration, TimeDirection::Forward)
            .end_action(EndActionKind::LeaveAsIs)
            .make()
    }
}
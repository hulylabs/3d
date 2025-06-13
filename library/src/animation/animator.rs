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

    pub(crate) fn take_time(&mut self) {
        self.current_time = Instant::now();
    }

    pub(crate) fn launch_morphing(&mut self, target: ObjectUid, parameters: ClockAnimationAct<PhaseAlive>) {
        self.animations.insert(target, Clock::new(self.current_time, parameters));
    }

    pub(crate) fn end(&mut self, target: ObjectUid) {
        self.animations.remove(&target);
    }

    pub(crate) fn clear_objects(&mut self) {
        self.animations.clear();
    }
    
    #[must_use]
    pub(crate) fn local_time_of(&self, global_clock: Instant, target: ObjectUid) -> f64 {
        if let Some(animation) = self.animations.get(&target) {
            animation.local_time(global_clock)
        } else {
            0.0
        }
    }
}

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

    pub(crate) fn clean_finished(&mut self) {
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

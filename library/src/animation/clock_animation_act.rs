use more_asserts::assert_gt;
use std::marker::PhantomData;
use std::time::Duration;

#[derive(PartialEq, Debug, Clone)]
pub struct PhaseConstruction;
#[derive(PartialEq, Debug, Clone)]
pub struct PhaseAlive;

#[derive(Clone, PartialEq, Debug)]
pub struct ClockAnimationAct<Phase = PhaseConstruction> {
    birth_time_offset: Duration,
    time_to_live: Option<LifeSpan>,
    local_playback_speed_multiplier: f64,
    periodization: Option<Periodization>,
    end_action: EndActionKind,

    phase: PhantomData<Phase>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeSpan {
    span: Duration,
    direction: TimeDirection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeDirection {
    Forward,
    Backward,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Periodization {
    wrap_kind: WrapKind,
    period: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WrapKind {
    Restart,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndActionKind {
    TeleportToZero,
    LeaveAsIs,
    TeleportToEnd,
}

impl LifeSpan {
    #[must_use]
    pub fn new(span: Duration, direction: TimeDirection) -> Self {
        Self { span, direction }
    }
    #[must_use]
    pub fn span(&self) -> Duration {
        self.span
    }
    #[must_use]
    pub fn reverse(&self) -> bool {
        TimeDirection::Backward == self.direction
    }
}

impl Periodization {
    #[must_use]
    pub fn new(wrap_kind: WrapKind, period: Duration) -> Self {
        Self { wrap_kind, period }
    }

    #[must_use]
    pub fn wrap_kind(&self) -> WrapKind {
        self.wrap_kind
    }

    #[must_use]
    pub fn period(&self) -> Duration {
        self.period
    }
}

impl ClockAnimationAct<PhaseConstruction> {
    const INFINITY: Option<LifeSpan> = None;

    #[must_use]
    pub fn make(self) -> ClockAnimationAct<PhaseAlive> {
        if let Some(ttl) = self.time_to_live {
            if ttl.direction == TimeDirection::Backward {
                assert_gt!(ttl.span, self.birth_time_offset)
            }
        }
        
        ClockAnimationAct::<PhaseAlive> {
            birth_time_offset: self.birth_time_offset,
            time_to_live: self.time_to_live,
            local_playback_speed_multiplier: self.local_playback_speed_multiplier,
            periodization: self.periodization,
            end_action: self.end_action,

            phase: PhantomData,
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            birth_time_offset: Duration::ZERO,
            time_to_live: Self::INFINITY,
            local_playback_speed_multiplier: 1.0,
            periodization: None,
            end_action: EndActionKind::LeaveAsIs,

            phase: PhantomData,
        }
    }

    #[must_use]
    pub fn birth_time_offset(mut self, value: Duration) -> Self {
        self.birth_time_offset = value;
        self
    }

    #[must_use]
    pub fn with_global_finite_time_to_live(mut self, span: Duration, direction: TimeDirection) -> Self {
        self.time_to_live = Some(LifeSpan{span, direction});
        self
    }

    #[must_use]
    pub fn with_global_infinite_time_to_live(mut self) -> Self {
        self.time_to_live = None;
        self
    }

    #[must_use]
    pub fn playback_speed_multiplier(mut self, multiplier: f64) -> Self {
        assert!(multiplier > 0.0, "playback speed multiplier must be positive, got {}", multiplier);
        self.local_playback_speed_multiplier = multiplier;
        self
    }

    #[must_use]
    pub fn periodization(mut self, value: Option<Periodization>) -> Self {
        self.periodization = value;
        self
    }

    #[must_use]
    pub fn end_action(mut self, action: EndActionKind) -> Self {
        self.end_action = action;
        self
    }
}

impl<Phase> ClockAnimationAct<Phase> {
    #[must_use]
    pub(crate) fn get_birth_time_offset(&self) -> Duration {
        self.birth_time_offset
    }

    #[must_use]
    pub(crate) fn get_time_to_live(&self) -> Option<LifeSpan> {
        self.time_to_live
    }

    #[must_use]
    pub(crate) fn get_playback_speed_multiplier(&self) -> f64 {
        self.local_playback_speed_multiplier
    }

    #[must_use]
    pub(crate) fn get_periodization(&self) -> Option<Periodization> {
        self.periodization
    }

    #[must_use]
    pub(crate) fn get_end_action(&self) -> EndActionKind {
        self.end_action
    }
}

impl Default for ClockAnimationAct<PhaseConstruction> {
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ClockAnimationAct<PhaseAlive> {
    #[must_use]
    fn default() -> Self {
        ClockAnimationAct::<PhaseConstruction>::new().make()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default_values() {
        let system_under_test = ClockAnimationAct::new();
        let default_blank = ClockAnimationAct::<PhaseConstruction>::default();
        let default_alive = ClockAnimationAct::<PhaseAlive>::default();

        assert_eq!(system_under_test                                , default_blank);
        assert_eq!(system_under_test.get_birth_time_offset()        , Duration::ZERO);
        assert_eq!(system_under_test.get_time_to_live()             , None);
        assert_eq!(system_under_test.get_playback_speed_multiplier(), 1.0);
        assert_eq!(system_under_test.get_periodization()            , None);
        assert_eq!(system_under_test.get_end_action()               , EndActionKind::LeaveAsIs);
        
        assert_eq!(default_alive.get_birth_time_offset()            , Duration::ZERO);
        assert_eq!(default_alive.get_time_to_live()                 , None);
        assert_eq!(default_alive.get_playback_speed_multiplier()    , 1.0);
        assert_eq!(default_alive.get_periodization()                , None);
        assert_eq!(default_alive.get_end_action()                   , EndActionKind::LeaveAsIs);
    }

    #[test]
    fn test_builder_with_custom_values() {
        let expected_birth_time_offset = Duration::from_secs(5);
        let expected_speed_multiplier = 2.0;
        let expected_ttl = Duration::from_secs(10);
        let expected_ttl_direction = TimeDirection::Backward;
        let expected_periodization = Some(Periodization::new(WrapKind::Reverse, Duration::from_secs(7)));
        let expected_end_action = EndActionKind::LeaveAsIs;

        let system_under_test = ClockAnimationAct::new()
            .birth_time_offset(expected_birth_time_offset)
            .with_global_finite_time_to_live(expected_ttl, expected_ttl_direction)
            .playback_speed_multiplier(expected_speed_multiplier)
            .periodization(expected_periodization)
            .end_action(expected_end_action)
            ;
        
        assert_eq!(system_under_test.get_birth_time_offset(), expected_birth_time_offset);
        assert_eq!(system_under_test.get_time_to_live(), Some(LifeSpan::new(expected_ttl, expected_ttl_direction)));
        assert_eq!(system_under_test.get_playback_speed_multiplier(), expected_speed_multiplier);
        assert_eq!(system_under_test.get_periodization(), expected_periodization);
        assert_eq!(system_under_test.get_end_action(), expected_end_action);
    }

    #[test]
    fn test_infinite_duration() {
        let system_under_test = ClockAnimationAct::new()
            .with_global_infinite_time_to_live();

        assert_eq!(system_under_test.get_time_to_live(), None);
    }

    #[test]
    #[should_panic(expected = "playback speed multiplier must be positive, got -1")]
    fn test_negative_multiplier_panics() {
        let _ = ClockAnimationAct::new().playback_speed_multiplier(-1.0);
    }

    #[test]
    #[should_panic(expected = "playback speed multiplier must be positive, got 0")]
    fn test_zero_multiplier_not_allowed() {
        let _ = ClockAnimationAct::new().playback_speed_multiplier(0.0);
    }

    #[test]
    fn test_clone() {
        let system_under_test = ClockAnimationAct::new()
            .birth_time_offset(Duration::from_secs(1))
            .with_global_finite_time_to_live(Duration::from_secs(2), TimeDirection::Backward)
            .playback_speed_multiplier(1.5)
            .periodization(Some(Periodization::new(WrapKind::Reverse, Duration::from_secs(7))))
            .end_action(EndActionKind::LeaveAsIs)
            ;

        let clone = system_under_test.clone();
        
        assert_eq!(clone, system_under_test);
    }

    #[test]
    #[should_panic]
    fn test_too_big_start_offset() {
        let life_span = Duration::from_secs(7);
        let time_offset = life_span + Duration::from_nanos(1);

        let _ = ClockAnimationAct::new()
            .with_global_finite_time_to_live(life_span, TimeDirection::Backward)
            .birth_time_offset(time_offset)
            .make();
    }
}
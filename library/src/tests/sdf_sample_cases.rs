#[cfg(test)]
pub(crate) mod tests {
    use crate::geometry::alias::Point;
    use crate::serialization::pod_vector::PodVector;

    #[must_use]
    fn to_gpu_format(position: Point) -> PodVector {
        PodVector::new(position.x as f32, position.y as f32, position.z as f32)
    }
    
    pub(crate) struct SdfSampleCases<T> {
        expected_outcomes: Vec<T>,
        sample_positions: Vec<PodVector>,
    }

    impl<T> SdfSampleCases<T> {
        #[must_use]
        pub(crate) fn new() -> Self {
            Self {
                expected_outcomes: Vec::new(),
                sample_positions: Vec::new(),
            }
        }

        pub(crate) fn add_case(&mut self, position_x: f64, position_y: f64, position_z: f64, expected_outcome: T) {
            self.sample_positions.push(to_gpu_format(Point::new(position_x, position_y, position_z)));
            self.expected_outcomes.push(expected_outcome);
        }

        pub(crate) fn add_case_point(&mut self, position: Point, expected_outcome: T) {
            self.add_case(position.x, position.y, position.z, expected_outcome);
        }

        #[must_use]
        pub(crate) fn expected_outcomes(&self) -> &[T] {
            &self.expected_outcomes
        }

        #[must_use]
        pub(crate) fn sample_positions(&self) -> &[PodVector] {
            &self.sample_positions
        }
    }

    impl<T> Default for SdfSampleCases<T> {
        fn default() -> Self {
            Self::new()
        }
    }

    #[test]
    fn test_construction() {
        let system_under_test = SdfSampleCases::<f32>::new();
        assert_eq!(system_under_test.expected_outcomes().len(), 0);
        assert_eq!(system_under_test.sample_positions().len(), 0);
    }

    #[test]
    fn test_add_sample() {
        let mut system_under_test = SdfSampleCases::<f32>::new();
        let position = Point::new(1.0, 2.0, 3.0);
        let expected_distance = 5.0;

        system_under_test.add_case_point(position.clone(), expected_distance);

        assert_eq!(system_under_test.expected_outcomes()[0], expected_distance);
        assert_eq!(system_under_test.sample_positions()[0], to_gpu_format(position));
    }

    #[test]
    fn test_multiple_samples() {
        let mut system_under_test = SdfSampleCases::<f32>::new();

        let pos_one = Point::new(1.0, 0.0, 0.0);
        let pos_two = Point::new(0.0, 1.0, 0.0);
        
        system_under_test.add_case_point(pos_one.clone(), 1.5);
        system_under_test.add_case_point(pos_two.clone(), 2.5);
        
        assert_eq!(system_under_test.expected_outcomes(), &[1.5, 2.5,]);
        assert_eq!(system_under_test.sample_positions(), &[to_gpu_format(pos_one), to_gpu_format(pos_two),]);
    }
}

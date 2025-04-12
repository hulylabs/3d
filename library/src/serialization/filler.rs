pub(crate) trait GpuFloatBufferFiller {
    fn write_and_move_next(&mut self, value: f64, iterator: &mut usize);

    const FLOAT_ALIGNMENT_SIZE: usize = 4;
    const PAD_VALUE: f32 = -1.0;

    fn pad_to_align(&mut self, iterator: &mut usize);
}

impl GpuFloatBufferFiller for [f32]  {
    fn write_and_move_next(&mut self, value: f64, iterator: &mut usize) {
        if *iterator >= self.len() {
            panic!("index out of bounds");
        }
        self[*iterator] = value as f32;
        *iterator += 1;
    }

    fn pad_to_align(&mut self, iterator: &mut usize) {
        if *iterator >= self.len() {
            panic!("index out of bounds");
        }
        let alignment = Self::FLOAT_ALIGNMENT_SIZE;
        let elements_to_pad = (alignment - (*iterator % alignment)) % alignment;
        for _ in 0..elements_to_pad {
            self[*iterator] = Self::PAD_VALUE;
            *iterator += 1;
        }
    }
}

#[must_use]
pub(crate) const fn floats_count(quartet_size: usize) -> usize {
    quartet_size * <[f32] as GpuFloatBufferFiller>::FLOAT_ALIGNMENT_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_move_next() {
        let mut system_under_test = [0.0; 8];
        let mut iterator = 0;

        system_under_test.write_and_move_next(1.5, &mut iterator);
        system_under_test.write_and_move_next(2.5, &mut iterator);

        assert_eq!(iterator, 2);
        assert_eq!(system_under_test[0], 1.5);
        assert_eq!(system_under_test[1], 2.5);
    }

    #[test]
    fn test_write_last_and_move_next() {
        let mut system_under_test = [0.0; 1];
        let mut iterator = 0;

        system_under_test.write_and_move_next(1.5, &mut iterator);

        assert_eq!(iterator, 1);
        assert_eq!(system_under_test[0], 1.5);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_write_out_of_bounds() {
        let mut system_under_test = [0.0; 2];
        let mut iterator = 2;

        system_under_test.write_and_move_next(3.0, &mut iterator);
    }

    #[test]
    fn test_pad_quartet_rest_filing() {
        let initial_filler_value = -1.0;
        let iterator_start = 5;
        let mut system_under_test = [initial_filler_value; 8];
        let mut iterator = iterator_start;

        system_under_test.pad_to_align(&mut iterator);

        assert_eq!(iterator, 8);

        for i in 0..iterator_start {
            assert_eq!(system_under_test[i], initial_filler_value);
        }

        assert_eq!(system_under_test[5], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
        assert_eq!(system_under_test[6], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
        assert_eq!(system_under_test[7], <[f32] as GpuFloatBufferFiller>::PAD_VALUE);
    }

    #[test]
    fn test_pad_quartet_rest_no_filing() {
        let initial_filler_value = -1.0;
        let iterator_start = 4;
        const BUFFER_LENGTH: usize = 8;
        let mut system_under_test = [initial_filler_value; BUFFER_LENGTH];
        let mut iterator = iterator_start;

        system_under_test.pad_to_align(&mut iterator);

        assert_eq!(iterator, iterator_start);
        for i in 0..BUFFER_LENGTH {
            assert_eq!(system_under_test[i], initial_filler_value);
        }
    }
}

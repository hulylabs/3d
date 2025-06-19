use crate::geometry::transform::Affine;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;

pub(crate) fn serialize_matrix_4x4(container: &mut GpuReadySerializationBuffer, matrix: &Affine) {
    assert!(container.free_quartets_of_current_object() >= 4);

    container.write_quartet_f64(
        matrix.x.x,
        matrix.x.y,
        matrix.x.z,
        matrix.x.w,
    );

    container.write_quartet_f64(
        matrix.y.x,
        matrix.y.y,
        matrix.y.z,
        matrix.y.w,
    );

    container.write_quartet_f64(
        matrix.z.x,
        matrix.z.y,
        matrix.z.z,
        matrix.z.w,
    );

    container.write_quartet_f64(
        matrix.w.x,
        matrix.w.y,
        matrix.w.z,
        matrix.w.w,
    );
}

pub(crate) fn serialize_matrix_3x4(container: &mut GpuReadySerializationBuffer, matrix: &Affine) {
    assert!(container.free_quartets_of_current_object() >= 4);

    container.write_quartet_f64(
        matrix.x.x,
        matrix.y.x,
        matrix.z.x,
        matrix.w.x,
    );

    container.write_quartet_f64(
        matrix.x.y,
        matrix.y.y,
        matrix.z.y,
        matrix.w.y,
    );

    container.write_quartet_f64(
        matrix.x.z,
        matrix.y.z,
        matrix.z.z,
        matrix.w.z,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::cast_slice;
    use crate::geometry::transform::constants::MATRIX_FLOATS_COUNT;

    #[test]
    fn test_serialize_matrix() {
        let test_matrix = Affine::new(
            1.0, 2.0, 3.0, 4.0,
            5.0, 6.0, 7.0, 8.0,
            9.0, 10.0, 11.0, 12.0,
            13.0, 14.0, 15.0, 16.0
        );

        let mut container = GpuReadySerializationBuffer::new(1, 4);

        serialize_matrix_4x4(&mut container, &test_matrix);
        let serialized: &[f32] = cast_slice(&container.backend());

        for i in 0..MATRIX_FLOATS_COUNT {
            assert_eq!(serialized[i], i as f32 + 1.0);
        }
    }
}
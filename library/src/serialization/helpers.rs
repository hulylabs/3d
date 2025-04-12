use crate::geometry::transform::Affine;
use crate::serialization::filler::GpuFloatBufferFiller;

pub(crate) const MATRIX_FLOATS_COUNT: usize = 16;

pub(crate) fn serialize_matrix(container: &mut [f32], matrix: &Affine, mut index: &mut usize) {
    assert!(container.len() >= MATRIX_FLOATS_COUNT);

    container.write_and_move_next(matrix.x.x, &mut index);
    container.write_and_move_next(matrix.x.y, &mut index);
    container.write_and_move_next(matrix.x.z, &mut index);
    container.write_and_move_next(matrix.x.w, &mut index);

    container.write_and_move_next(matrix.y.x, &mut index);
    container.write_and_move_next(matrix.y.y, &mut index);
    container.write_and_move_next(matrix.y.z, &mut index);
    container.write_and_move_next(matrix.y.w, &mut index);

    container.write_and_move_next(matrix.z.x, &mut index);
    container.write_and_move_next(matrix.z.y, &mut index);
    container.write_and_move_next(matrix.z.z, &mut index);
    container.write_and_move_next(matrix.z.w, &mut index);

    container.write_and_move_next(matrix.w.x, &mut index);
    container.write_and_move_next(matrix.w.y, &mut index);
    container.write_and_move_next(matrix.w.z, &mut index);
    container.write_and_move_next(matrix.w.w, &mut index);
}

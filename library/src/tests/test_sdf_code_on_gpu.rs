#[cfg(test)]
mod tests {
    use crate::geometry::alias::{Point, Vector};
    use crate::sdf::code_generator::{SdfCodeGenerator, SdfRegistrator};
    use crate::sdf::named_sdf::{NamedSdf, UniqueName};
    use crate::sdf::sdf::Sdf;
    use crate::sdf::sdf_box::SdfBox;
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::sdf::sdf_union::SdfUnion;
    use crate::sdf::shader_function_name::FunctionName;
    use crate::serialization::pod_vector::PodVector;
    use std::fmt::Write;
    use std::rc::Rc;
    use cgmath::Deg;
    use crate::sdf::sdf_box_frame::SdfBoxFrame;
    use crate::sdf::sdf_capped_torus_xy::SdfCappedTorusXy;
    use crate::sdf::sdf_cone::SdfCone;
    use crate::sdf::sdf_hex_prism::SdfHexPrism;
    use crate::sdf::sdf_link::SdfLink;
    use crate::sdf::sdf_round_box::SdfRoundBox;
    use crate::sdf::sdf_torus_xz::SdfTorusXz;
    use crate::tests::assert_utils::tests::assert_eq;
    use crate::tests::gpu_code_execution::tests::execute_code;

    #[test]
    fn test_sdf_union_spheres() {
        let union = SdfUnion::new(
            SdfSphere::new_offset(2.0, Point::new(0.0,  7.0, 0.0)),
            SdfSphere::new_offset(2.0, Point::new(0.0, -7.0, 0.0)),
        );

        let input_points = [
            PodVector { x: 0.0, y: 0.0 , z: 0.0 , w: 0.0 },
            
            PodVector { x: 0.0 , y: 5.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 7.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 9.0 , z: 0.0 , w: 0.0 },

            PodVector { x: 0.0 , y: -5.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: -7.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: -9.0 , z: 0.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
             5.0_f32,

             0.0_f32,
            -2.0_f32,
             0.0_f32,

             0.0_f32,
            -2.0_f32,
             0.0_f32,
        ];

        test_sdf_evaluation(union, "spheres_union", &input_points, &expected_signed_distances);
    }
    
    #[test]
    fn test_sdf_sphere() {
        let sphere = SdfSphere::new(17.0);

        let input_points = [
            PodVector { x: 13.0, y: 0.0 , z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 17.0, z: 0.0 , w: 0.0 },
            PodVector { x: 0.0 , y: 0.0 , z: 23.0, w: 0.0 },
        ];

        let expected_signed_distances = [
            -4.0_f32, 0.0_f32, 6.0_f32,
        ];
        
        test_sdf_evaluation(sphere, "sphere", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_sdf_box() {
        let a_box = SdfBox::new(Vector::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  2.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  3.0 , w: 0.0 },

            PodVector { x: -1.0 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y: -2.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z: -3.0 , w: 0.0 },

            PodVector { x:  0.7 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  1.7 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  2.7 , w: 0.0 },

            PodVector { x:  1.1 , y:  2.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.2 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.0 , z:  5.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
             0.0_f32,  0.0_f32,  0.0_f32,
             0.0_f32,  0.0_f32,  0.0_f32,

            -0.3_f32, -0.3_f32, -0.3_f32,
             0.1_f32,  0.2_f32,  2.0_f32,
        ];

        test_sdf_evaluation(a_box, "box", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_sdf_round_box() {
        let radius = 0.2;
        let a_box = SdfRoundBox::new(Vector::new(1.0, 2.0, 3.0), radius);

        let input_points = [
            PodVector { x:  1.0 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  2.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  3.0 , w: 0.0 },

            PodVector { x:  0.7 , y:  0.0 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  1.7 , z:  0.0 , w: 0.0 },
            PodVector { x:  0.0 , y:  0.0 , z:  2.7 , w: 0.0 },

            PodVector { x:  1.0 , y:  2.0 , z:  3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
             0.0_f32,  0.0_f32,  0.0_f32,

            -0.3_f32, -0.3_f32, -0.3_f32,

            0.14641021_f32,
        ];

        test_sdf_evaluation(a_box, "box", &input_points, &expected_signed_distances);
    }

    #[test]
    fn tes_box_frame() {
        let box_frame = SdfBoxFrame::new_offset(Vector::new(1.0, 2.0, 3.0), 0.1, Point::new(-1.0, -1.0, -1.0));

        let input_points = [
            PodVector { x:  -1.0 , y:  -1.0 , z:  -1.0 , w: 0.0 },
            PodVector { x:   0.0 , y:   1.0 , z:   2.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
            1.9697715_f32,
            0.0_f32,
        ];

        test_sdf_evaluation(box_frame, "box_frame", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_torus_xz() {
        let minor_radius = 5.0;
        let major_radius = 7.0;
        let torus = SdfTorusXz::new_offset(major_radius, minor_radius, Point::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:   3.0 , w: 0.0 },
            PodVector { x:  6.0 , y:  2.0 , z:   3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.0 , z:  10.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  2.0 , z:   9.0 , w: 0.0 },
            PodVector { x:  3.0 , y:  2.0 , z:   3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
             2.0_f32,
            -3.0_f32,
            -5.0_f32,
            -4.0_f32,
             0.0_f32,
        ];

        test_sdf_evaluation(torus, "torus", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_capped_torus_xy() {
        let minor_radius = 1.0;
        let major_radius = 2.0;
        let capped_torus = SdfCappedTorusXy::new_offset(Deg(180.0), major_radius, minor_radius, Point::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:  3.0 , w: 0.0 },
            
            PodVector { x:  1.0 , y:  1.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  0.0 , z:  3.0 , w: 0.0 },
            
            PodVector { x:  1.0 , y:  3.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  4.0 , z:  3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
             1.0_f32,
             0.0_f32, 
            -1.0_f32,
             0.0_f32,
            -1.0_f32,
        ];

        test_sdf_evaluation(capped_torus, "capped_torus", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_link() {
        let outer_radius = 1.0;
        let inner_radius = 2.0;
        let half_length = 4.0;
        let link = SdfLink::new_offset(half_length, inner_radius, outer_radius, Point::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:  3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
            1.0_f32,
        ];

        test_sdf_evaluation(link, "link", &input_points, &expected_signed_distances);
    }

    #[test]
    fn test_cone() {
        let height = 5.0;
        let link = SdfCone::new_offset(Deg(45.0), height, Point::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:  2.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  3.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  7.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  8.0 , z:  3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
            0.0_f32,
            1.0_f32,
            5.0_f32,
            6.0_f32,
        ];

        test_sdf_evaluation(link, "cone", &input_points, &expected_signed_distances);
    }
    
    #[test]
    fn test_hex_prism() {
        let width = 7.0;
        let height = 5.0;
        let link = SdfHexPrism::new_offset(width, height, Point::new(1.0, 2.0, 3.0));

        let input_points = [
            PodVector { x:  1.0 , y:   2.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  8.0 , y:   3.0 , z:  3.0 , w: 0.0 },
            PodVector { x:  1.0 , y:  12.0 , z:  3.0 , w: 0.0 },
        ];

        let expected_signed_distances = [
            -5.0_f32,
            -0.43782234_f32,
             3.0_f32,
        ];

        test_sdf_evaluation(link, "hex_prism", &input_points, &expected_signed_distances);
    }
    
    fn test_sdf_evaluation(sdf: Rc<dyn Sdf>, name: &str, sample_positions: &[PodVector], expected_distances: &[f32]) {
        let named = NamedSdf::new(sdf, UniqueName::new(name.to_string()));

        let mut registrator = SdfRegistrator::new();
        registrator.add(&named);

        let generator = SdfCodeGenerator::new(registrator);

        let mut shader_code: String = String::new();
        let function_to_call = generator.generate_unique_code_for(&named, &mut shader_code);
        generator.generate_shared_code(&mut shader_code);

        let actual_distances = execute_function(&sample_positions, function_to_call, shader_code);

        let epsilon = 1e-6;
        assert_eq(&actual_distances, expected_distances, epsilon);
    }
    
    #[must_use]
    fn execute_function(input: &[PodVector], function_name: FunctionName, function_code: String) -> Vec<f32> {
        let mut function_execution = make_function_execution(function_name);
        function_execution.write_str(function_code.as_str()).expect("shader code concatenation has failed");

        execute_code(input, function_execution.as_str())
    }

    const FUNCTION_EXECUTOR: &str = include_str!("point_function_executor.wgsl");

    #[must_use]
    fn make_function_execution(name: FunctionName) -> String {
        FUNCTION_EXECUTOR.to_string().replace("_FUNCTION_NAME_SLOT_TO_BE_FILLED_", name.0.as_str())
    }
}
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

        let expected_signed_distances: Vec<f32> = vec![
             5.0,
             
             0.0,
            -2.0,
             0.0,

             0.0, 
            -2.0,
             0.0,
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

        let expected_signed_distances: Vec<f32> = vec![
            -4.0, 0.0, 6.0
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

        let expected_signed_distances: Vec<f32> = vec![
             0.0,  0.0,  0.0,
             0.0,  0.0,  0.0,
             
            -0.3, -0.3, -0.3,
             0.1,  0.2,  2.0,
        ];

        test_sdf_evaluation(a_box, "box", &input_points, &expected_signed_distances);
    }
    
    fn test_sdf_evaluation(sdf: Rc<dyn Sdf>, name: &str, sample_positions: &[PodVector], expected_distances: &Vec<f32>) {
        let named = NamedSdf::new(sdf, UniqueName(name.to_string()));

        let mut registrator = SdfRegistrator::new();
        registrator.add(&named);

        let generator = SdfCodeGenerator::new(registrator);

        let mut shader_code: String = String::new();
        let function_to_call = generator.generate_unique_code_for(&named, &mut shader_code);
        generator.generate_shared_code(&mut shader_code);

        let actual_distances = execute_function(&sample_positions, function_to_call, shader_code);

        let epsilon = 1e-7;
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
#[cfg(test)]
mod tests {
    use crate::geometry::alias::Vector;
    use crate::scene::sdf_warehouse::SdfWarehouse;
    use crate::sdf::code_generator::SdfRegistrator;
    use crate::sdf::named_sdf::{NamedSdf, UniqueName};
    use crate::sdf::sdf_box::SdfBox;
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::serialization::pod_vector::PodVector;
    use crate::tests::assert_utils::tests::assert_eq;
    use crate::tests::common::tests::COMMON_GPU_EVALUATIONS_EPSILON;
    use crate::tests::gpu_code_execution::tests::execute_code;
    use std::fmt::Write;

    #[test]
    fn test_sdf_selection_evaluation() {
        let sphere = NamedSdf::new(SdfSphere::new(17.0), UniqueName::new("identity_sphere".to_string()));
        let a_box = NamedSdf::new(SdfBox::new(Vector::new(2.0, 3.0, 5.0)), UniqueName::new("some_box".to_string()));
        
        let mut registrator = SdfRegistrator::new();
        registrator.add(&sphere);
        registrator.add(&a_box);
        
        let warehouse = SdfWarehouse::new(registrator);

        let sphere_index = warehouse.index_for_name(sphere.name()).unwrap();
        let box_index = warehouse.index_for_name(a_box.name()).unwrap();

        let mut the_code = warehouse.sdf_classes_code().to_string();
        the_code.write_str(FUNCTION_EXECUTOR).expect("shader code concatenation has failed");

        let input_points = [
            PodVector { x:    0.0 , y:  0.0 , z:  0.0 , w: sphere_index.as_f64() as f32, },
            PodVector { x:  - 3.0 , y:  0.0 , z:  0.0 , w: sphere_index.as_f64() as f32, },
            PodVector { x:  -17.0 , y:  0.0 , z:  0.0 , w: sphere_index.as_f64() as f32, },
            PodVector { x:  -19.0 , y:  0.0 , z:  0.0 , w: sphere_index.as_f64() as f32, },

            PodVector { x:    0.0 , y:  0.0 , z:  0.0 , w: box_index.as_f64() as f32, },
            PodVector { x:    2.0 , y:  3.0 , z:  5.0 , w: box_index.as_f64() as f32, },
            PodVector { x:    8.0 , y:  3.0 , z:  5.0 , w: box_index.as_f64() as f32, },
        ];

        let expected_distances: Vec<f32> = vec![
            -17.0,
            -14.0,
              0.0,
              2.0,
            
            - 2.0,
              0.0,
              6.0,
        ];

        let actual_distances = execute_code(&input_points, the_code.as_str());
        
        assert_eq(&actual_distances, &expected_distances, COMMON_GPU_EVALUATIONS_EPSILON);
    }
    
    const FUNCTION_EXECUTOR: &str = include_str!("sdf_function_executor.wgsl");
}
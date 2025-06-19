#[cfg(test)]
mod tests {
    use crate::container::sdf_warehouse::SdfWarehouse;
    use crate::geometry::alias::Vector;
    use crate::sdf::framework::code_generator::SdfRegistrator;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::sdf::object::sdf_sphere::SdfSphere;
    use crate::serialization::pod_vector::PodVector;
    use crate::tests::gpu_code_execution::tests::{execute_code, ExecutionConfig};
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};
    use crate::utils::tests::assert_utils::tests::assert_eq;
    use crate::utils::tests::common_values::tests::COMMON_GPU_EVALUATIONS_EPSILON;
    use std::fmt::Write;

    #[test]
    fn test_sdf_selection_evaluation() {
        let sphere = NamedSdf::new(SdfSphere::new(17.0), UniqueSdfClassName::new("identity_sphere".to_string()));
        let a_box = NamedSdf::new(SdfBox::new(Vector::new(2.0, 3.0, 5.0)), UniqueSdfClassName::new("some_box".to_string()));
        
        let mut registrator = SdfRegistrator::new();
        registrator.add(&sphere);
        registrator.add(&a_box);
        
        let warehouse = SdfWarehouse::new(registrator);

        let sphere_index = warehouse.properties_for_name(sphere.name()).unwrap();
        let box_index = warehouse.properties_for_name(a_box.name()).unwrap();

        let template = ShaderFunction::new("vec4f", "f32", "sdf_select")
            .with_additional_shader_code(warehouse.sdf_classes_code().to_string());

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

        let function_execution = make_executable(&template,create_argument_formatter!("{argument}.w, {argument}.xyz, 0.0"));

        let actual_distances = execute_code(&input_points, function_execution, ExecutionConfig::default());
        
        assert_eq(&actual_distances, &expected_distances, COMMON_GPU_EVALUATIONS_EPSILON);
    }
}
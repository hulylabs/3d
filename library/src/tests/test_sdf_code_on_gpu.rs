#[cfg(test)]
mod tests {
    use crate::geometry::aabb::Aabb;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::axis::Axis;
    use crate::sdf::code_generator::{SdfCodeGenerator, SdfRegistrator};
    use crate::sdf::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::sdf_base::Sdf;
    use crate::sdf::sdf_box::SdfBox;
    use crate::sdf::sdf_box_frame::SdfBoxFrame;
    use crate::sdf::sdf_capped_cylinder_along_axis::SdfCappedCylinderAlongAxis;
    use crate::sdf::sdf_capped_torus_xy::SdfCappedTorusXy;
    use crate::sdf::sdf_capsule::SdfCapsule;
    use crate::sdf::sdf_cone::SdfCone;
    use crate::sdf::sdf_cut_hollow_sphere::SdfCutHollowSphere;
    use crate::sdf::sdf_hex_prism::SdfHexPrism;
    use crate::sdf::sdf_link::SdfLink;
    use crate::sdf::sdf_octahedron::SdfOctahedron;
    use crate::sdf::sdf_pyramid::SdfPyramid;
    use crate::sdf::sdf_round_box::SdfRoundBox;
    use crate::sdf::sdf_round_cone::SdfRoundCone;
    use crate::sdf::sdf_solid_angle::SdfSolidAngle;
    use crate::sdf::sdf_sphere::SdfSphere;
    use crate::sdf::sdf_torus_xz::SdfTorusXz;
    use crate::sdf::sdf_triangular_prism::SdfTriangularPrism;
    use crate::sdf::sdf_union::SdfUnion;
    use crate::sdf::sdf_vesica_segment::SdfVesicaSegment;
    use crate::sdf::shader_function_name::FunctionName;
    use crate::serialization::pod_vector::PodVector;
    use crate::utils::tests::assert_utils::tests::assert_eq;
    use crate::tests::gpu_code_execution::tests::{execute_code, ExecutionConfig};
    use crate::tests::sdf_sample_cases::tests::SdfSampleCases;
    use crate::tests::shader_entry_generator::tests::{create_argument_formatter, make_executable, ShaderFunction};
    use cgmath::{Deg, EuclideanSpace, InnerSpace};
    use more_asserts::{assert_ge, assert_gt};
    use std::fmt::Write;
    use std::rc::Rc;
    use crate::geometry::epsilon::DEFAULT_EPSILON_F32;
    use crate::sdf::sdf_translation::SdfTranslation;
    use crate::utils::tests::common_values::tests::COMMON_GPU_EVALUATIONS_EPSILON;

    #[test]
    fn test_sdf_union_spheres() {
        let system_under_test = SdfUnion::new(
            SdfTranslation::new(Vector::new(0.0,  7.0, 0.0), SdfSphere::new(2.0)),
            SdfTranslation::new(Vector::new(0.0, -7.0, 0.0), SdfSphere::new(2.0)),
        );

        let mut test_cases = SdfSampleCases::<f32>::new();
        
        test_cases.add_case(0.0,  0.0, 0.0,  5.0_f32);
        
        test_cases.add_case(0.0,  5.0, 0.0,  0.0_f32);
        test_cases.add_case(0.0,  7.0, 0.0, -2.0_f32);
        test_cases.add_case(0.0,  9.0, 0.0,  0.0_f32);
        
        test_cases.add_case(0.0, -5.0, 0.0,  0.0_f32);
        test_cases.add_case(0.0, -7.0, 0.0, -2.0_f32);
        test_cases.add_case(0.0, -9.0, 0.0,  0.0_f32);

        test_sdf_evaluation(system_under_test, "spheres_union", &test_cases);
    }
    
    #[test]
    fn test_sdf_sphere() {
        let system_under_test = SdfSphere::new(17.0);

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(13.0,  0.0 , 0.0 ,-4.0_f32);
        test_cases.add_case(0.0 , 17.0 , 0.0 , 0.0_f32);
        test_cases.add_case(0.0 ,  0.0 , 23.0, 6.0_f32);

        test_sdf_evaluation(system_under_test, "sphere", &test_cases);
    }

    #[test]
    fn test_sdf_box() {
        let system_under_test = SdfBox::new(Vector::new(1.0, 2.0, 3.0));

        let mut test_cases = SdfSampleCases::<f32>::new();
        
        test_cases.add_case( 1.0,  0.0,  0.0,  0.0_f32);
        test_cases.add_case( 0.0,  2.0,  0.0,  0.0_f32);
        test_cases.add_case( 0.0,  0.0,  3.0,  0.0_f32);
        test_cases.add_case(-1.0,  0.0,  0.0,  0.0_f32);
        test_cases.add_case( 0.0, -2.0,  0.0,  0.0_f32);
        test_cases.add_case( 0.0,  0.0, -3.0,  0.0_f32);
        
        test_cases.add_case( 0.7,  0.0,  0.0, -0.3_f32);
        test_cases.add_case( 0.0,  1.7,  0.0, -0.3_f32);
        test_cases.add_case( 0.0,  0.0,  2.7, -0.3_f32);
        test_cases.add_case( 1.1,  2.0,  3.0,  0.1_f32);
        test_cases.add_case( 1.0,  2.2,  3.0,  0.2_f32);
        test_cases.add_case( 1.0,  2.0,  5.0,  2.0_f32);
        
        test_sdf_evaluation(system_under_test, "box", &test_cases);
    }

    #[test]
    fn test_sdf_round_box() {
        let radius = 0.2;
        let system_under_test = SdfRoundBox::new(Vector::new(1.0, 2.0, 3.0), radius);

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 0.0,0.0, 0.0_f32       );
        test_cases.add_case(0.0, 2.0,0.0, 0.0_f32       );
        test_cases.add_case(0.0, 0.0,3.0, 0.0_f32       );
        test_cases.add_case(0.7, 0.0,0.0,-0.3_f32       );
        test_cases.add_case(0.0, 1.7,0.0,-0.3_f32       );
        test_cases.add_case(0.0, 0.0,2.7,-0.3_f32       );
        test_cases.add_case(1.0, 2.0,3.0, 0.14641021_f32);

        test_sdf_evaluation(system_under_test, "box", &test_cases);
    }

    #[test]
    fn tes_box_frame() {
        let system_under_test =
            SdfTranslation::new(Vector::new(-1.0, -1.0, -1.0), SdfBoxFrame::new(Vector::new(1.0, 2.0, 3.0), 0.1));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(-1.0 , -1.0 , -1.0, 1.9697715_f32);
        test_cases.add_case( 0.0 ,  1.0 ,  2.0, 0.0_f32);

        test_sdf_evaluation(system_under_test, "box_frame", &test_cases);
    }

    #[test]
    fn test_torus_xz() {
        let minor_radius = 5.0;
        let major_radius = 7.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfTorusXz::new(major_radius, minor_radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0,  3.0,  2.0_f32);
        test_cases.add_case(6.0, 2.0,  3.0, -3.0_f32);
        test_cases.add_case(1.0, 2.0, 10.0, -5.0_f32);
        test_cases.add_case(1.0, 2.0,  9.0, -4.0_f32);
        test_cases.add_case(3.0, 2.0,  3.0,  0.0_f32);

        test_sdf_evaluation(system_under_test, "torus", &test_cases);
    }

    #[test]
    fn test_capped_torus_xy() {
        let minor_radius = 1.0;
        let major_radius = 2.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfCappedTorusXy::new(Deg(180.0), major_radius, minor_radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0, 3.0,  1.0_f32);
        test_cases.add_case(1.0, 1.0, 3.0,  0.0_f32);
        test_cases.add_case(1.0, 0.0, 3.0, -1.0_f32);
        test_cases.add_case(1.0, 3.0, 3.0,  0.0_f32);
        test_cases.add_case(1.0, 4.0, 3.0, -1.0_f32);

        test_sdf_evaluation(system_under_test, "capped_torus", &test_cases);
    }

    #[test]
    fn test_link() {
        let outer_radius = 1.0;
        let inner_radius = 2.0;
        let half_length = 4.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfLink::new(half_length, inner_radius, outer_radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0,2.0, 3.0, 1.0_f32);

        test_sdf_evaluation(system_under_test, "link", &test_cases);
    }

    #[test]
    fn test_cone() {
        let height = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfCone::new(Deg(45.0), height));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0, 3.0, 0.0_f32);
        test_cases.add_case(1.0, 3.0, 3.0, 1.0_f32);
        test_cases.add_case(1.0, 7.0, 3.0, 5.0_f32);
        test_cases.add_case(1.0, 8.0, 3.0, 6.0_f32);

        test_sdf_evaluation(system_under_test, "cone", &test_cases);
    }
    
    #[test]
    fn test_hex_prism() {
        let width = 7.0;
        let height = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfHexPrism::new(width, height));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0 ,3.0, -5.0_f32       );
        test_cases.add_case(8.0, 3.0 ,3.0, -0.43782234_f32);
        test_cases.add_case(1.0, 12.0,3.0,  3.0_f32       );

        test_sdf_evaluation(system_under_test, "hex_prism", &test_cases);
    }
    
    #[test]
    fn test_triangular_prism() {
        let width = 3.0;
        let height = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(3.0, 2.0, 1.0), SdfTriangularPrism::new(width, height));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(3.0, 2.0, 1.0, -1.5_f32     );
        test_cases.add_case(3.0, 2.0, 2.0, -1.5_f32     );
        test_cases.add_case(3.0, 3.0, 1.0, -1.0_f32     );
        test_cases.add_case(4.0, 2.0, 1.0, -0.633975_f32);

        test_sdf_evaluation(system_under_test, "hex_prism", &test_cases);
    }
    
    
    #[test]
    fn test_capsule() {
        let start = Point::new(0.0, 0.0, -1.0);
        let end = Point::new(0.0, 0.0, 1.0);
        let radius = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(3.0, 5.0, 7.0), SdfCapsule::new(start, end, radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(3.0, 5.0, 7.0 , -5.0_f32);
        test_cases.add_case(3.0, 5.0, 13.0,  0.0_f32);
        test_cases.add_case(3.0, 5.0, 1.0 ,  0.0_f32);

        test_sdf_evaluation(system_under_test, "capsule", &test_cases);
    }

    #[test]
    fn test_capped_cylinder_y() {
        let half_height = 19.0;
        let radius = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(3.0, 5.0, 7.0), SdfCappedCylinderAlongAxis::new(Axis::Y, half_height, radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(3.0,   5.0, 7.0, -5.0_f32);
        test_cases.add_case(3.0,   5.0 ,2.0,  0.0_f32);
        test_cases.add_case(3.0,  24.0 ,7.0,  0.0_f32);
        test_cases.add_case(3.0, -14.0 ,7.0,  0.0_f32);
        
        test_sdf_evaluation(system_under_test, "capped_cylinder_y", &test_cases);
    }
    
    #[test]
    fn test_solid_angle() {
        let radius = 5.0;
        let system_under_test = SdfTranslation::new(Vector::new(3.0, 5.0, 7.0), SdfSolidAngle::new(Deg(45.0), radius));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(3.0, 5.0, 7.0, 0.0_f32          );
        test_cases.add_case(3.0, 6.0, 7.0, -std::f32::consts::FRAC_1_SQRT_2);
        
        test_sdf_evaluation(system_under_test, "solid_angle", &test_cases);
    }
    
    #[test]
    fn test_cut_hollow_sphere() {
        let radius = 13.0;
        let cut_height = 6.0;
        let thickness = 1.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfCutHollowSphere::new(radius, cut_height, thickness));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0, 3.0 , 12.0_f32,);
        test_cases.add_case(1.0, 2.0, 16.0, -1.0_f32,);
        test_cases.add_case(1.0, 2.0, 17.0,  0.0_f32,);
    
        test_sdf_evaluation(system_under_test, "cut_hollow_sphere", &test_cases);
    }

    #[test]
    fn test_round_cone() {
        let height = 7.0;
        let radius_major = 2.0;
        let radius_minor = 1.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfRoundCone::new(radius_major, radius_minor, height));

        let mut test_cases = SdfSampleCases::<f32>::new();
        
        test_cases.add_case(1.0, 2.0,  3.0, -2.0_f32       );
        test_cases.add_case(1.0, 3.0 , 3.0, -1.8571428_f32 );
        test_cases.add_case(1.0, 9.0 , 3.0, -1.0_f32       );
        test_cases.add_case(1.0, 10.0, 3.0,  0.0_f32       );
        
        test_sdf_evaluation(system_under_test, "round_cone", &test_cases);
    }
    
    #[test]
    fn test_vesica_segment() {
        let width = 7.0;
        let start = Point::new(3.0, 0.0, 0.0);
        let end = Point::new(0.0, 7.0, 0.0);
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfVesicaSegment::new(width, start, end));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0, 3.0, -1.893274_f32,);
        test_cases.add_case(3.0, 0.0, 0.0,  0.808540_f32,);
        test_cases.add_case(0.0, 7.0, 0.0, -1.974256_f32,);
        
        test_sdf_evaluation(system_under_test, "vesica_segment", &test_cases);
    }
    
    #[test]
    fn test_octahedron() {
        let size = 1.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfOctahedron::new(size));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0, 2.0, 3.0, -0.57735026_f32,);
        test_cases.add_case(1.0, 4.0, 3.0,  1.0_f32,);
        test_cases.add_case(1.0, 3.0, 3.0,  0.0_f32,);
        test_cases.add_case(3.0, 2.0, 3.0,  1.0_f32,);
        test_cases.add_case(1.0, 2.0, 6.0,  2.0_f32,);
    
        test_sdf_evaluation(system_under_test, "octahedron", &test_cases);
    }
    
    #[test]
    fn test_pyramid() {
        let size = 13.0;
        let system_under_test = SdfTranslation::new(Vector::new(1.0, 2.0, 3.0), SdfPyramid::new(size));

        let mut test_cases = SdfSampleCases::<f32>::new();
        test_cases.add_case(1.0  , 2.0  , 3.0, 0.0);
        test_cases.add_case(1.0  , 15.0  , 3.0, 0.0);

        test_sdf_evaluation(system_under_test, "pyramid", &test_cases);
    }

    fn test_sdf_evaluation(sdf: Rc<dyn Sdf>, name: &str, test_cases: &SdfSampleCases<f32>) {
        let named = NamedSdf::new(sdf, UniqueSdfClassName::new(name.to_string()));

        let mut registrator = SdfRegistrator::new();
        registrator.add(&named);

        let generator = SdfCodeGenerator::new(registrator);

        let mut sdf_shader_code: String = String::new();
        let function_to_call = generator.generate_unique_code_for(&named, &mut sdf_shader_code);
        generator.generate_shared_code(&mut sdf_shader_code);

        let actual_distances = execute_function(&test_cases.sample_positions(), &function_to_call, &sdf_shader_code);
        assert_eq(&actual_distances, test_cases.expected_outcomes(), COMMON_GPU_EVALUATIONS_EPSILON);

        check_sdf_values_around_aabb(&named, &sdf_shader_code, &function_to_call);
    }

    fn check_sdf_values_around_aabb(named: &NamedSdf, sdf_shader_code: &String, function_to_call: &FunctionName) {
        let aabb = named.sdf().aabb();
        let mut test_data = SdfSampleCases::<RelativeOutcome>::new();
        
        test_data.add_case(aabb.min().x, aabb.min().y, aabb.min().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.min().x, aabb.min().y, aabb.max().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.min().x, aabb.max().y, aabb.max().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.max().x, aabb.max().y, aabb.max().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.max().x, aabb.max().y, aabb.min().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.max().x, aabb.min().y, aabb.min().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.max().x, aabb.min().y, aabb.max().z, RelativeOutcome::BorderOrOutside);
        test_data.add_case(aabb.min().x, aabb.max().y, aabb.min().z, RelativeOutcome::BorderOrOutside);

        {
            let center = aabb.center();
            test_data.add_case(center.x, center.y, aabb.min().z, RelativeOutcome::BorderOrOutside);
            test_data.add_case(center.x, center.y, aabb.max().z, RelativeOutcome::BorderOrOutside);
            
            test_data.add_case(aabb.min().x, center.y, center.z, RelativeOutcome::BorderOrOutside);
            test_data.add_case(aabb.max().x, center.y, center.z, RelativeOutcome::BorderOrOutside);
            
            test_data.add_case(center.x, aabb.min().y, center.z, RelativeOutcome::BorderOrOutside);
            test_data.add_case(center.x, aabb.max().y, center.z, RelativeOutcome::BorderOrOutside);
        }
        
        // move slightly outside the corners of aabb
        let corners = test_data.sample_positions().to_vec();
        for corner in corners {
            let corner_point = Point::new(corner.x as f64, corner.y as f64, corner.z as f64);
            let offset = (corner_point - aabb.center()).normalize() * aabb.extent().magnitude() * 0.001;
            test_data.add_case_point(corner_point + offset, RelativeOutcome::Outside);
        }

        let actual_distances = execute_function(test_data.sample_positions(), &function_to_call, &sdf_shader_code);
        for i in 0..actual_distances.len() {
            match test_data.expected_outcomes()[i] {
                RelativeOutcome::BorderOrOutside => {
                    assert_ge!(actual_distances[i] + DEFAULT_EPSILON_F32, 0.0, "distance index == {}", i);
                }
                RelativeOutcome::Outside => {
                    assert_gt!(actual_distances[i], 0.0, "distance index == {}", i);
                }
            }
        }
    }

    enum RelativeOutcome {
        BorderOrOutside,
        Outside,
    }

    impl Aabb {
        #[must_use]
        pub(crate) fn center(&self) -> Point {
            Point::from_vec((self.max().to_vec() + self.min().to_vec()) * 0.5)
        }
    }

    #[must_use]
    fn execute_function(input: &[PodVector], function_name: &FunctionName, function_code: &String) -> Vec<f32> {
        let template = ShaderFunction::new("vec4f", "f32", function_name.0.as_str())
            .with_additional_shader_code(function_code.as_str());
        
        let function_execution = make_executable(&template, create_argument_formatter!("{argument}.xyz"));

        execute_code(input, function_execution, ExecutionConfig::default())
    }
}
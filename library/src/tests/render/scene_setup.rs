#[cfg(test)]
pub(crate) mod tests {
    use std::{env, fs};
    use std::ops::Deref;
    use std::path::{Path, PathBuf};
    use cgmath::Deg;
    use palette::Srgb;
    use crate::container::visual_objects::VisualObjects;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::transform::Affine;
    use crate::gpu::color_buffer_evaluation::RenderStrategyId;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_vulkan_context;
    use crate::gpu::render::{FrameBufferSettings, Renderer};
    use crate::gpu::render::tests::{out_folder_path, save_colors_to_png, shoot_rays_and_transfer_data_to_cpu, test_folder_path};
    use crate::material::material_properties::{MaterialClass, MaterialProperties};
    use crate::palette::sdf::sdf_box_frame::SdfBoxFrame;
    use crate::palette::sdf::sdf_capsule::SdfCapsule;
    use crate::palette::sdf::sdf_round_box::SdfRoundBox;
    use crate::palette::sdf::sdf_torus_xz::SdfTorusXz;
    use crate::scene::camera::Camera;
    use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
    use crate::sdf::framework::sdf_registrator::SdfRegistrator;
    use crate::sdf::object::sdf_box::SdfBox;
    use crate::sdf::object::sdf_sphere::SdfSphere;
    use crate::tests::render::utils::tests::compare_png_images;
    use crate::utils::file_system::ensure_folders_exist;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;

    const TEST_FRAME_BUFFER_WIDTH: u32 = 512;
    const TEST_FRAME_BUFFER_HEIGHT: u32 = 512;
    const TEST_FRAME_BUFFER_SIZE: FrameBufferSize = FrameBufferSize::new(TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT);

    const TEST_ANTI_ALIASING_LEVEL: u32 = 3;
    const TEST_REFRACTIVE_INDEX: f64 = 1.8;
    const TEST_SPECULAR_STRENGTH: f64 = 0.5;

    #[must_use]
    fn do_we_have_cli_flag_on(flag: &str) -> bool {
        let arguments: Vec<String> = env::args().collect();
        let flag_variants = [
            flag,
            &format!("--{}", flag),
            &format!("-{}", flag),
        ];

        arguments.iter().any(|argument| flag_variants.contains(&argument.as_str()))
    }

    #[must_use]
    fn make_new_reference_mode() -> bool {
        do_we_have_cli_flag_on("make_new_reference")
    }

    fn copy_to_reference<FilePath: AsRef<Path>>(
        source_path: FilePath,
        destination_path: FilePath,
    ) -> Result<(), Box<dyn std::error::Error>> {
        ensure_folders_exist(&destination_path)?;
        fs::copy(&source_path, &destination_path)?;
        Ok(())
    }

    #[must_use]
    fn add_suffix_to_filename(path: &PathBuf, suffix: &str) -> PathBuf {
        let mut new_path = path.clone();

        if let Some(stem) = path.file_stem() {
            let new_filename = if let Some(ext) = path.extension() {
                format!("{}{}.{}", stem.to_string_lossy(), suffix, ext.to_string_lossy())
            } else {
                format!("{}{}", stem.to_string_lossy(), suffix)
            };
            new_path.set_file_name(new_filename);
        }

        new_path
    }

    #[test]
    fn test_add_suffix_to_filename() {
        let actual_path = add_suffix_to_filename(&PathBuf::from("test.png"), "_test");
        assert_eq!(actual_path.to_string_lossy(), "test_test.png");

        let actual_path = add_suffix_to_filename(&PathBuf::from("foo").join("test.png"), "_test");
        assert_eq!(actual_path.to_string_lossy(), PathBuf::from("foo").join("test_test.png").to_string_lossy());
    }
    
    pub(crate) fn compare_test_scene_to_reference(
        sender_strategy: RenderStrategyId,
        light_color: Srgb,
        material_customization: MaterialClass,
        test_case_name: &PathBuf,
    ) {
        let context = create_headless_wgpu_vulkan_context();

        let look_at = Point::new(0.0, 0.0, 0.0);
        let mut camera = Camera::new_perspective_camera(3.0, look_at);
        camera.move_horizontally(-0.2);
        camera.move_vertically(-0.2);

        let mut registrator = SdfRegistrator::default();

        let identity_box_sdf = UniqueSdfClassName::new("box_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfBox::new(Vector::new(0.5, 0.5, 0.5)), identity_box_sdf.clone()));
        let identity_sphere_sdf = UniqueSdfClassName::new("sphere_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfSphere::new(1.0), identity_sphere_sdf.clone()));
        let frame_box_sdf = UniqueSdfClassName::new("box_frame_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfBoxFrame::new(Vector::new(0.5, 0.1, 0.2), 0.02), frame_box_sdf.clone()));

        let round_box_sdf = UniqueSdfClassName::new("round_box_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfRoundBox::new(Vector::new(0.1, 0.4, 0.2), 0.05), round_box_sdf.clone()));
        let torus_xz_sdf = UniqueSdfClassName::new("torus_xz_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfTorusXz::new(0.1, 0.05), torus_xz_sdf.clone()));
        let capsule_sdf = UniqueSdfClassName::new("capsule_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfCapsule::new(Point::new(-1.0, -1.0, -1.0), Point::new(1.0, 1.0, 1.0), 0.3), capsule_sdf.clone()));

        let mut scene = VisualObjects::new(None, Some(registrator), None);

        let emissive_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_emission(light_color.red, light_color.green, light_color.blue));

        let white_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(1.0, 1.0, 1.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );
        let gray_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(0.5, 0.5, 0.5)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );
        let yellow_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(1.0, 1.0, 0.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_class(material_customization)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );
        let magenta_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(1.0, 0.0, 1.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_class(material_customization)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );
        let cyan_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(0.0, 1.0, 1.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(4.0)
            .with_class(material_customization)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );
        let bright_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_albedo(4.0, 3.0, 2.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_class(material_customization)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX)
        );

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-1.0, -1.0, 0.9)) * Affine::from_scale(0.5)),
            1.0, &identity_sphere_sdf, magenta_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-1.0, -1.0, -0.2)) * Affine::from_scale(0.4)),
            1.0, &identity_box_sdf, white_material);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-1.0, 1.0, 0.0)) * Affine::from_scale(0.2)),
            1.0, &identity_sphere_sdf, magenta_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.0, 1.0, 0.0)) * Affine::from_nonuniform_scale(0.2, 0.1, 0.3)),
            1.0, &identity_box_sdf, yellow_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 1.0, 0.0)) * Affine::from_nonuniform_scale(0.4, 0.4, 1.5)),
            0.1, &frame_box_sdf, gray_material);

        scene.add_sdf(
            &(Affine::from_angle_x(Deg(45.0))),
            1.0, &round_box_sdf, white_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-1.0, 0.0, 0.0)) * Affine::from_angle_z(Deg(60.0))),
            1.0, &torus_xz_sdf, cyan_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.0, 0.0, 0.0)) * Affine::from_angle_y(Deg(145.0)) * Affine::from_scale(0.2)),
            1.0, &capsule_sdf, bright_material);

        scene.add_parallelogram(Point::new(-2.0, -2.0, -1.0), Vector::new(4.0, 0.0, 0.0), Vector::new(0.0, 4.0, 0.0), white_material);
        scene.add_parallelogram(Point::new(-1.0, -2.0, -1.0), Vector::new(0.0, 0.0, 1.0), Vector::new(2.0, 0.0, 0.0), emissive_material);

        let frame_buffer_settings = FrameBufferSettings::new(COMMON_PRESENTATION_FORMAT, TEST_FRAME_BUFFER_SIZE, TEST_ANTI_ALIASING_LEVEL);
        let mut system_under_test
            = Renderer::new(context.clone(), scene, camera, frame_buffer_settings, sender_strategy, None)
            .expect("render instantiation has failed");

        shoot_rays_and_transfer_data_to_cpu(context.deref(), &mut system_under_test);

        let actual_render_path = out_folder_path().join(test_case_name.clone());

        save_colors_to_png(&mut system_under_test, TEST_FRAME_BUFFER_SIZE, actual_render_path.clone());

        let expected_render_path = test_folder_path().join("reference").join(test_case_name);
        let diff_path = out_folder_path().join("difference").join(add_suffix_to_filename(test_case_name, "_diff"));

        if make_new_reference_mode() {
            copy_to_reference(&actual_render_path, &expected_render_path)
                .expect("failed to copy new reference file");
            println!("new reference file created: {:?}", expected_render_path);
        } else {
            let equal = compare_png_images(actual_render_path, expected_render_path, diff_path)
                .expect("render output comparison failed");
            assert!(equal, "render output differs from the reference");
        }
    }
}
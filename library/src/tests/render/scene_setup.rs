#[cfg(test)]
pub(crate) mod tests {
    use crate::container::texture_helpers::load_bitmap;
    use crate::container::visual_objects::VisualObjects;
    use crate::geometry::alias::{Point, Vector};
    use crate::geometry::transform::Affine;
    use crate::gpu::color_buffer_evaluation::RenderStrategyId;
    use crate::gpu::context::Context;
    use crate::gpu::frame_buffer_size::FrameBufferSize;
    use crate::gpu::headless_device::tests::create_headless_wgpu_vulkan_context;
    use crate::gpu::render::tests::{data_folder_path, out_folder_path, save_colors_to_png, shoot_rays_and_transfer_data_to_cpu, test_folder_path};
    use crate::gpu::render::{FrameBufferSettings, Renderer};
    use crate::material::atlas_region_mapping::{AtlasRegionMappingBuilder, WrapMode};
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
    use crate::tests::render::images_comparison::tests::{add_suffix_to_filename, copy_to_reference, make_new_reference_mode};
    use crate::tests::render::utils::tests::compare_png_images;
    use crate::utils::bitmap_utils::BitmapSize;
    use crate::utils::tests::common_values::tests::COMMON_PRESENTATION_FORMAT;
    use cgmath::{Deg, Vector4};
    use palette::Srgb;
    use std::ops::Deref;
    use std::path::PathBuf;
    use std::rc::Rc;

    const TEST_FRAME_BUFFER_WIDTH: u32 = 512;
    const TEST_FRAME_BUFFER_HEIGHT: u32 = 512;
    const TEST_FRAME_BUFFER_SIZE: FrameBufferSize = FrameBufferSize::new(TEST_FRAME_BUFFER_WIDTH, TEST_FRAME_BUFFER_HEIGHT);

    const TEST_ANTI_ALIASING_LEVEL: u32 = 3;
    const TEST_REFRACTIVE_INDEX: f64 = 1.8;
    const TEST_SPECULAR_STRENGTH: f64 = 0.5;
    
    pub(crate) fn compare_test_objects_scene_render_to_reference(
        render_strategy: RenderStrategyId,
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
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
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

        render_and_compare_with_reference(render_strategy, test_case_name, context, camera, scene);
    }

    pub(crate) fn compare_test_textures_scene_render_to_reference(
        render_strategy: RenderStrategyId,
        light_color: Srgb,
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
        let torus_xz_sdf = UniqueSdfClassName::new("torus_xz_specimen".to_string());
        registrator.add(&NamedSdf::new(SdfTorusXz::new(0.1, 0.05), torus_xz_sdf.clone()));

        let mut scene = VisualObjects::new(Some(BitmapSize::new(256, 512)), Some(registrator), None);

        let texture_atlas_page_composer = scene.mutable_texture_atlas_page_composer();
        let checkerboard_texture = load_bitmap(data_folder_path().join("bitmap_checkerboard_small.png"), texture_atlas_page_composer)
            .expect("failed to load bitmap_checkerboard_small.png");
        let logo_texture = load_bitmap(data_folder_path().join("bitmap_huly_2.png"), texture_atlas_page_composer)
            .expect("failed to load bitmap_huly_2.png");
        let grid_texture = load_bitmap(data_folder_path().join("bitmap_rect_grid.png"), texture_atlas_page_composer)
            .expect("failed to load bitmap_rect_grid.png");

        let emissive_material = scene.materials_mutable().add(&MaterialProperties::new()
            .with_emission(light_color.red, light_color.green, light_color.blue));

        let mut white_material_properties = MaterialProperties::new()
            .with_albedo(1.0, 1.0, 1.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX);
        {
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(1.1, 0.0, 0.0, 0.0))
                .local_position_to_texture_v(Vector4::new(0.0, 1.1, 0.0, 0.0))
                .wrap_mode([WrapMode::Repeat, WrapMode::Repeat])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(checkerboard_texture, builder, &mut white_material_properties)
                .expect("failed to make 'bitmap_checkerboard_small' atlas page mapping");
        }
        let white_material = scene.materials_mutable().add(&white_material_properties);

        let mut yellow_material_properties = MaterialProperties::new()
            .with_albedo(1.0, 1.0, 0.0)
            .with_specular(1.0, 1.0, 1.0)
            .with_specular_strength(TEST_SPECULAR_STRENGTH)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX);
        {
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(0.5, 0.0, 0.0, 0.5))
                .local_position_to_texture_v(Vector4::new(0.0, -0.9, 0.0, 0.5))
                .wrap_mode([WrapMode::Clamp, WrapMode::Clamp])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(logo_texture, builder, &mut yellow_material_properties)
                .expect("failed to make 'bitmap_huly_2' atlas page mapping");
        }
        let yellow_material = scene.materials_mutable().add(&yellow_material_properties);

        let mut magenta_material_properties = MaterialProperties::new()
            .with_albedo(1.0, 0.0, 1.0)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX);
        {
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(2.0, 0.0, 0.0, 0.0))
                .local_position_to_texture_v(Vector4::new(0.0, 2.0, 0.0, 0.0))
                .wrap_mode([WrapMode::Repeat, WrapMode::Discard])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(checkerboard_texture, builder, &mut magenta_material_properties)
                .expect("failed to make 'bitmap_checkerboard_small' atlas page mapping");
        }
        let magenta_material = scene.materials_mutable().add(&magenta_material_properties);

        let mut cyan_material_properties = MaterialProperties::new()
            .with_albedo(0.0, 1.0, 1.0)
            .with_refractive_index_eta(TEST_REFRACTIVE_INDEX);
        {
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(4.0, 0.0, 0.0, 0.5))
                .local_position_to_texture_v(Vector4::new(0.0, -8.0, 0.0, 0.5))
                .wrap_mode([WrapMode::Clamp, WrapMode::Repeat])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(grid_texture, builder, &mut cyan_material_properties)
                .expect("failed to make 'bitmap_rect_grid' atlas page mapping");
        }
        let cyan_material = scene.materials_mutable().add(&cyan_material_properties);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, -0.8, 0.0)) * Affine::from_scale(0.5)),
            1.0, &identity_sphere_sdf, yellow_material);
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.8, -0.8, -0.2)) * Affine::from_scale(0.4)),
            1.0, &identity_box_sdf, magenta_material);
        scene.add_sdf(
            &( Affine::from_translation(Vector::new(0.8, 0.8, 0.0)) * Affine::from_angle_x(Deg(90.0)) * Affine::from_scale(2.0) ),
            1.0, &torus_xz_sdf, cyan_material);

        scene.add_parallelogram(Point::new(-2.0, -2.0, -1.0), Vector::new(4.0, 0.0, 0.0), Vector::new(0.0, 4.0, 0.0), white_material);
        scene.add_parallelogram(Point::new(-1.0, 0.0, 1.0), Vector::new(0.0, 0.2, 0.0), Vector::new(2.0, 0.0, 0.0), emissive_material);

        render_and_compare_with_reference(render_strategy, test_case_name, context, camera, scene);
    }

    fn render_and_compare_with_reference(render_strategy: RenderStrategyId, test_case_name: &PathBuf, context: Rc<Context>, camera: Camera, scene: VisualObjects) {
        let frame_buffer_settings = FrameBufferSettings::new(COMMON_PRESENTATION_FORMAT, TEST_FRAME_BUFFER_SIZE, TEST_ANTI_ALIASING_LEVEL);
        let mut system_under_test
            = Renderer::new(context.clone(), scene, camera, frame_buffer_settings, render_strategy, None)
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
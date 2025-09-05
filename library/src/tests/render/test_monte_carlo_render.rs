#[cfg(test)]
mod tests {

    mod monte_carlo_render {
        use std::path::PathBuf;
        use crate::gpu::color_buffer_evaluation::RenderStrategyId;

        const RENDER_STRATEGY: RenderStrategyId = RenderStrategyId::MonteCarlo;

        #[must_use]
        fn folder() -> PathBuf {
            PathBuf::from("monte_carlo")
        }
        
        mod opaque_objects {
            use palette::Srgb;
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_scene_to_reference;
            use crate::tests::render::test_monte_carlo_render::tests::monte_carlo_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;

            #[test]
            fn test_with_perspective_camera_magenta_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Lambert, 
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_with_perspective_camera_yellow_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Lambert, 
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_with_perspective_camera_cyan_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Lambert, 
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_with_perspective_camera_white_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Lambert, 
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_white_light")));
            }
        }

        mod mirror_objects {
            use palette::Srgb;
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_scene_to_reference;
            use crate::tests::render::test_monte_carlo_render::tests::monte_carlo_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;

            #[test]
            fn test_with_perspective_camera_magenta_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_with_perspective_camera_yellow_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_with_perspective_camera_cyan_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_with_perspective_camera_white_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_white_light")));
            }
        }


        mod glass_objects {
            use palette::Srgb;
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_scene_to_reference;
            use crate::tests::render::test_monte_carlo_render::tests::monte_carlo_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;

            #[test]
            fn test_glass_objects_with_perspective_camera_magenta_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_yellow_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_cyan_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_white_light() {
                compare_test_scene_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_white_light")));
            }
        }
    }
}
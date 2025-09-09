#[cfg(test)]
mod tests {

    mod deterministic_render {
        use crate::gpu::color_buffer_evaluation::RenderStrategyId;
        use std::path::PathBuf;

        const RENDER_STRATEGY: RenderStrategyId = RenderStrategyId::Deterministic;

        #[must_use]
        fn folder() -> PathBuf {
            PathBuf::from("deterministic")  
        } 
        
        mod opaque_objects {
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_objects_scene_render_to_reference;
            use crate::tests::render::test_deterministic_render::tests::deterministic_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;
            use palette::Srgb;
        
            #[test]
            fn test_with_perspective_camera_magenta_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Lambert,
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_with_perspective_camera_yellow_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Lambert,
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_with_perspective_camera_cyan_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Lambert,
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_with_perspective_camera_white_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Lambert,
                    &folder().join(make_png_file_name("opaque_objects_perspective_camera_white_light")));
            }
        }
        
        mod mirror_objects {
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_objects_scene_render_to_reference;
            use crate::tests::render::test_deterministic_render::tests::deterministic_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;
            use palette::Srgb;
        
            #[test]
            fn test_with_perspective_camera_magenta_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_with_perspective_camera_yellow_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_with_perspective_camera_cyan_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_with_perspective_camera_white_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Mirror,
                    &folder().join(make_png_file_name("mirror_objects_perspective_camera_white_light")));
            }
        }
        
        
        mod glass_objects {
            use crate::material::material_properties::MaterialClass;
            use crate::tests::render::scene_setup::tests::compare_test_objects_scene_render_to_reference;
            use crate::tests::render::test_deterministic_render::tests::deterministic_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;
            use palette::Srgb;
        
            #[test]
            fn test_glass_objects_with_perspective_camera_magenta_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_magenta_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_yellow_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_yellow_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_cyan_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_cyan_light")));
            }
            #[test]
            fn test_glass_objects_with_perspective_camera_white_light() {
                compare_test_objects_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0), MaterialClass::Glass,
                    &folder().join(make_png_file_name("glass_objects_perspective_camera_white_light")));
            }
        }

        mod textured_objects {
            use crate::tests::render::scene_setup::tests::compare_test_textures_scene_render_to_reference;
            use crate::tests::render::test_deterministic_render::tests::deterministic_render::{folder, RENDER_STRATEGY};
            use crate::tests::render::utils::tests::make_png_file_name;
            use palette::Srgb;

            #[test]
            fn test_textures_with_magenta_light() {
                compare_test_textures_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 0.0, 1.0),
                    &folder().join(make_png_file_name("textures_with_magenta_light")));
            }
            #[test]
            fn test_textures_with_yellow_light() {
                compare_test_textures_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 0.0),
                    &folder().join(make_png_file_name("textures_with_yellow_light")));
            }
            #[test]
            fn test_textures_with_cyan_light() {
                compare_test_textures_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(0.0, 1.0, 1.0),
                    &folder().join(make_png_file_name("textures_with_cyan_light")));
            }
            #[test]
            fn test_textures_with_white_light() {
                compare_test_textures_scene_render_to_reference(RENDER_STRATEGY, Srgb::new(1.0, 1.0, 1.0),
                    &folder().join(make_png_file_name("textures_with_white_light")));
            }
        }
    }
}
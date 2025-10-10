use crate::container::visual_objects::VisualObjects;
use crate::geometry::aabb::Aabb;
use crate::geometry::alias::Vector;
use crate::geometry::transform::Affine;
use crate::material::material_index::MaterialIndex;
use crate::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
use crate::sdf::framework::sdf_registrator::SdfRegistrator;
use crate::sdf::object::reservation::make_reserved_name;
use crate::text::glyph_atlas_composer::GlyphAtlasComposer;
use crate::text::sdf_glyph::SdfGlyph;
use crate::text::tribal_knowledge::GLYPH_LOCAL_Z_EXTRUSION_THICKNESS;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use cgmath::{ElementWise, Vector2, Zero};
use crate::objects::common_properties::ObjectUid;
use crate::objects::sdf_instance::SdfPayload;
use crate::serialization::gpu_ready_serialization_buffer::GpuReadySerializationBuffer;
use crate::text::string_object_uid::StringObjectUid;
use crate::utils::bitmap_utils::BitmapSize;
use crate::utils::uid_generator::UidGenerator;

pub struct SceneStringPrologue {
    atlas_composer: GlyphAtlasComposer,
    glyph_sdf_name: UniqueSdfClassName,
}

impl SceneStringPrologue {
    const SDF_GLYPH_CLASS_NAME_SUFFIX: &'static str = "glyph";

    pub fn new(atlas_bitmap_file: impl AsRef<Path>, sdf_classes: &mut SdfRegistrator) -> anyhow::Result<Self> {
        let atlas_composer = GlyphAtlasComposer::new(atlas_bitmap_file)?;
        let glyph_sdf_name = UniqueSdfClassName::new(make_reserved_name(Self::SDF_GLYPH_CLASS_NAME_SUFFIX));

        if let Some(max_glyph_size) = atlas_composer.max_glyph_uv_size() {
            let aabb = Aabb::from_size(Vector::new(max_glyph_size.x, max_glyph_size.y, GLYPH_LOCAL_Z_EXTRUSION_THICKNESS));
            sdf_classes.add_no_name_check(&NamedSdf::new(SdfGlyph::new(aabb), glyph_sdf_name.clone()));
        }

        Ok(Self {
            atlas_composer,
            glyph_sdf_name,
        })
    }
}

pub struct SceneStrings {
    uid_generator: UidGenerator<StringObjectUid>,
    atlas_composer: GlyphAtlasComposer,
    scene: Rc<RefCell<VisualObjects>>,
    glyph_sdf_name: UniqueSdfClassName,
    strings: HashMap<StringObjectUid, Vec<ObjectUid>>
}

impl SceneStrings {
    const SDF_GLYPH_RAY_MARCHING_STEP_SCALE: f64 = 1.0;

    const Y_AXIS_INVERSION: Vector2<f64> = Vector2::<f64>::new(1.0, -1.0);

    pub fn new(scene: Rc<RefCell<VisualObjects>>, prologue: SceneStringPrologue) -> SceneStrings {
        Self {
            uid_generator: UidGenerator::new(),
            atlas_composer: prologue.atlas_composer,
            scene,
            glyph_sdf_name: prologue.glyph_sdf_name,
            strings: HashMap::new(),
        }
    }

    #[must_use]
    pub(crate) fn get_spares_of(&self, target: &StringObjectUid) -> Option<&Vec<ObjectUid>> {
        self.strings.get(target)
    }

    pub(crate) fn add(&mut self, text: &str, baseline_location: &Affine, material: MaterialIndex) -> StringObjectUid {
        let mut sdf_instances: Vec<ObjectUid> = vec![];
        let mut scene = self.scene.borrow_mut();

        let mut current_line_top = self.atlas_composer.line_baseline();
        let mut current_offset = 0.0;
        let mut previous_char: char = 0 as char;
        for char in text.chars() {
            if char == '\n' {
                current_line_top -= self.atlas_composer.line_height();
                current_offset = 0.0;
                previous_char = 0 as char;
                continue
            }

            if let Some(glyph) = self.atlas_composer.get_glyph(char) {
                let kerning = self.atlas_composer.get_glyph_kerning(char, previous_char).unwrap_or(0.0);
                current_offset += kerning;
                if char != ' ' {
                    let left_top
                        = Vector2::<f64>::new(current_offset, current_line_top)
                        + glyph.offset().mul_element_wise(Self::Y_AXIS_INVERSION);

                    let baseline_relative
                        = left_top
                        + glyph.size().mul_element_wise(Self::Y_AXIS_INVERSION) * 0.5;

                    let glyph_transform = baseline_location * Affine::from_translation(Vector::new(baseline_relative.x, baseline_relative.y, 0.0));
                    let payload = SdfPayload(glyph.index());
                    let ray_marching_step_scale = Self::SDF_GLYPH_RAY_MARCHING_STEP_SCALE;

                    let instance_uid = scene.add_sdf(&glyph_transform, ray_marching_step_scale, &self.glyph_sdf_name, material, payload);
                    sdf_instances.push(instance_uid);
                }
                current_offset += glyph.x_advance();
            } else {
                current_offset += self.atlas_composer.max_glyph_uv_size().unwrap_or(Vector2::zero()).y;
            }

            previous_char = char;
        }

        let string_uid = self.uid_generator.next();
        self.strings.insert(string_uid, sdf_instances);
        string_uid
    }

    #[must_use]
    pub(crate) fn glyph_atlas_size(&self) -> BitmapSize {
        self.atlas_composer.get_atlas_size()
    }

    #[must_use]
    pub(crate) fn glyph_atlas_layout(&self) -> GpuReadySerializationBuffer {
        self.atlas_composer.get_atlas_layout()
    }

    #[must_use]
    pub(crate) fn glyph_atlas_bitmap(&self) -> &[u8] {
        self.atlas_composer.get_atlas_bitmap()
    }

    #[must_use]
    pub(crate) fn texel_to_world_scale(&self) -> f64 {
        self.atlas_composer.generation_time_distance_range() * 0.0005
    }

    #[must_use]
    pub(crate) fn make_empty_atlas_layout_buffer() -> GpuReadySerializationBuffer {
        GlyphAtlasComposer::make_empty_atlas_layout_buffer()
    }
}

#[cfg(test)]
mod tests {
    use cgmath::SquareMatrix;
    use more_asserts::assert_gt;
    use test_context::{test_context, TestContext};
    use crate::container::visual_objects::DataKind;
    use crate::material::material_properties::MaterialProperties;
    use crate::text::tribal_knowledge::tests::ASCII_PRINTABLE_GLYPHS_COUNT;
    use crate::utils::tests::conventions::tests::data_folder_path;
    use super::*;

    struct Context {
        system_under_test: SceneStrings,
        dummy_material: MaterialIndex,
        scene: Rc<RefCell<VisualObjects>>,
    }

    impl TestContext for Context {
        fn setup() -> Context {
            let mut registrator = SdfRegistrator::new();
            let prologue = SceneStringPrologue::new(data_folder_path().join("glyphs_atlas_ascii_domine.png"), &mut registrator).unwrap();
            let scene = Rc::new(RefCell::new(VisualObjects::new(None, Some(registrator), None)));
            let dummy_material = scene.borrow_mut().materials_mutable().add(&MaterialProperties::default());
            let system_under_test = SceneStrings::new(scene.clone(), prologue);

            Context { system_under_test, dummy_material, scene }
        }

        fn teardown(self) {
        }
    }

    #[test_context(Context)]
    #[test]
    fn test_construction(fixture: &mut Context) {
        let expected_bitmap_size = BitmapSize::new(1024, 512);
        assert_eq!(fixture.system_under_test.glyph_atlas_size(), expected_bitmap_size);
        assert_eq!(fixture.system_under_test.glyph_atlas_layout().total_slots_count(), ASCII_PRINTABLE_GLYPHS_COUNT);
        assert_gt!(fixture.system_under_test.texel_to_world_scale(), 0.0);
        assert_eq!(fixture.system_under_test.glyph_atlas_bitmap().len(), expected_bitmap_size.bytes_in_bitmap());
    }

   fn test_sdf_instance_add(fixture: &mut Context, string: &str, expected_sdf_instances_count: usize) {
        fixture.system_under_test.add(string, &Affine::identity(), fixture.dummy_material);
        assert_eq!(fixture.scene.borrow().count_of_a_kind(DataKind::Sdf), expected_sdf_instances_count);
    }

    #[test_context(Context)]
    #[test]
    fn test_sdf_instance_single_letter_add(fixture: &mut Context) {
        let expected_sdf_instances_count: usize = 1;
        test_sdf_instance_add(fixture, "A", expected_sdf_instances_count);
    }


    #[test_context(Context)]
    #[test]
    fn test_sdf_instance_multiple_letters_add(fixture: &mut Context) {
        let expected_sdf_instances_count: usize = 10;
        let non_ascii_letter_to_be_skipped = 'Ё';
        test_sdf_instance_add(fixture, format!("AbCd_{non_ascii_letter_to_be_skipped}_FgHi").as_str(), expected_sdf_instances_count);
    }
}
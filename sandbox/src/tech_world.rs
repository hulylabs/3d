use cgmath::{Deg, Vector4};
use library::container::mesh_warehouse::MeshWarehouse;
use library::container::visual_objects::VisualObjects;
use library::geometry::alias::{Point, Vector};
use library::geometry::axis::Axis;
use library::geometry::transform::{Affine, Transformation};
use library::material::material_index::MaterialIndex;
use library::material::material_properties::{MaterialClass, MaterialProperties};
use library::material::procedural_texture_index::ProceduralTextureUid;
use library::material::procedural_textures::ProceduralTextures;
use library::material::texture_procedural_2d::TextureProcedural2D;
use library::material::texture_reference::TextureReference;
use library::palette::material::procedural_texture_checkerboard::make_checkerboard_texture;
use library::palette::sdf::sdf_box_frame::SdfBoxFrame;
use library::palette::sdf::sdf_capped_cylinder_along_axis::SdfCappedCylinderAlongAxis;
use library::palette::sdf::sdf_capped_torus_xy::SdfCappedTorusXy;
use library::palette::sdf::sdf_capsule::SdfCapsule;
use library::palette::sdf::sdf_cone::SdfCone;
use library::palette::sdf::sdf_cut_hollow_sphere::SdfCutHollowSphere;
use library::palette::sdf::sdf_hex_prism::SdfHexPrism;
use library::palette::sdf::sdf_link::SdfLink;
use library::palette::sdf::sdf_octahedron::SdfOctahedron;
use library::palette::sdf::sdf_pyramid::SdfPyramid;
use library::palette::sdf::sdf_rhombus::SdfRhombus;
use library::palette::sdf::sdf_round_box::SdfRoundBox;
use library::palette::sdf::sdf_round_cone::SdfRoundCone;
use library::palette::sdf::sdf_solid_angle::SdfSolidAngle;
use library::palette::sdf::sdf_torus_xz::SdfTorusXz;
use library::palette::sdf::sdf_triangular_prism::SdfTriangularPrism;
use library::palette::sdf::sdf_vesica_segment::SdfVesicaSegment;
use library::scene::hub::Hub;
use library::sdf::composition::sdf_intersection::SdfIntersection;
use library::sdf::composition::sdf_intersection_smooth::SdfIntersectionSmooth;
use library::sdf::composition::sdf_subtraction::SdfSubtraction;
use library::sdf::composition::sdf_subtraction_smooth::SdfSubtractionSmooth;
use library::sdf::composition::sdf_union::SdfUnion;
use library::sdf::composition::sdf_union_smooth::SdfUnionSmooth;
use library::sdf::framework::sdf_registrator::SdfRegistrator;
use library::sdf::framework::named_sdf::{NamedSdf, UniqueSdfClassName};
use library::sdf::morphing::sdf_bender_along_axis::SdfBenderAlongAxis;
use library::sdf::morphing::sdf_twister_along_axis::SdfTwisterAlongAxis;
use library::sdf::object::sdf_box::SdfBox;
use library::sdf::object::sdf_sphere::SdfSphere;
use library::sdf::transformation::sdf_translation::SdfTranslation;
use library::shader::code::{FunctionBody, Generic, ShaderCode};
use library::shader::conventions;
use library::utils::object_uid::ObjectUid;
use log::error;
use std::env;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use library::container::texture_atlas_page_composer::{AtlasRegionUid, TextureAtlasPageComposer};
use library::material::atlas_region_mapping::{AtlasRegionMappingBuilder, WrapMode};
use library::utils::bitmap_utils::{BitmapSize, ImmutableBitmapReference};

const CONTENT_ROOT_FOLDER_NAME: &str = "assets";

#[must_use]
fn get_resource_path(file_name: impl AsRef<Path>) -> PathBuf {
    let exe_path = env::current_exe().unwrap();
    let exe_directory = exe_path.parent().unwrap();
    exe_directory.join(file_name)
}

pub(super) struct TechWorldBitmapTextures {
    bitmap_checkerboard_small: AtlasRegionUid,
    bitmap_checkerboard_large: AtlasRegionUid,
    bitmap_huly: AtlasRegionUid,
    bitmap_huly_2: AtlasRegionUid,
    bitmap_rect_grid: AtlasRegionUid,
}

impl TechWorldBitmapTextures {
    pub(super) fn new(composer: &mut TextureAtlasPageComposer) -> anyhow::Result<Self> {
        let bitmap_checkerboard_small= TechWorldBitmapTextures::load_bitmap("bitmap_checkerboard_small.png", composer)?;
        let bitmap_checkerboard_large= TechWorldBitmapTextures::load_bitmap("bitmap_checkerboard_large.png", composer)?;
        let bitmap_huly= TechWorldBitmapTextures::load_bitmap("bitmap_huly.png", composer)?;
        let bitmap_huly_2= TechWorldBitmapTextures::load_bitmap("bitmap_huly_2.png", composer)?;
        let bitmap_rect_grid = TechWorldBitmapTextures::load_bitmap("bitmap_rect_grid.png", composer)?;

        composer.save_page_into(Path::new("debug_output").join("textures_atlas.png")).expect("failed to save texture atlas page");

        Ok(Self {
            bitmap_checkerboard_small,
            bitmap_checkerboard_large,
            bitmap_huly,
            bitmap_huly_2,
            bitmap_rect_grid,
        })
    }

    fn load_bitmap(file_name: &str, composer: &mut TextureAtlasPageComposer) -> anyhow::Result<AtlasRegionUid> {
        let image_path = Path::new(CONTENT_ROOT_FOLDER_NAME).join(file_name);
        let image
            = image::open(get_resource_path(&image_path))
                .map_err(|e| anyhow!("failed to open image {}: {}", image_path.display(), e))?;

        let buffer = image.to_rgba8();
        let bitmap_size = BitmapSize::new(buffer.width() as usize, buffer.height() as usize);

        composer
            .allocate(ImmutableBitmapReference::new(buffer.as_raw(), bitmap_size))
                .ok_or_else(|| anyhow!("failed to allocate region in texture atlas for {}", file_name))
    }
}

pub(super) struct TechWorldSdfClasses {
    rectangular_box: NamedSdf,
    sphere: NamedSdf,
    quad_0_3: NamedSdf,
    identity_box: NamedSdf,
    identity_box_frame: NamedSdf,
    xz_torus: NamedSdf,
    capped_xy_torus: NamedSdf,
    round_box: NamedSdf,
    link: NamedSdf,
    cone: NamedSdf,
    hex_prism: NamedSdf,
    triangular_prism: NamedSdf,
    capsule: NamedSdf,
    cylinder_cross: NamedSdf,
    solid_angle: NamedSdf,
    cut_hollow_sphere: NamedSdf,
    round_cone: NamedSdf,
    vesica_segment: NamedSdf,
    octahedron: NamedSdf,
    rhombus: NamedSdf,
    pyramid: NamedSdf,
    csg_example: NamedSdf,
    union_smooth: NamedSdf,
    subtraction_smooth: NamedSdf,
    intersection_smooth: NamedSdf,
    
    twisted_box: NamedSdf,
    bent_box: NamedSdf,
}

impl TechWorldSdfClasses {
    #[must_use]
    pub(super) fn new(registrator: &mut SdfRegistrator) -> Self {
        let rectangular_box = NamedSdf::new(SdfBox::new(Vector::new(0.24, 0.1, 0.24)), UniqueSdfClassName::new("button".to_string()));
        registrator.add(&rectangular_box);

        let quad_0_3= NamedSdf::new(SdfBox::new(Vector::new(0.15, 0.15, 0.005)), UniqueSdfClassName::new("quad_zero_three".to_string()));
        registrator.add(&quad_0_3);

        let sphere = NamedSdf::new(SdfSphere::new(1.0), UniqueSdfClassName::new("bubble".to_string()));
        registrator.add(&sphere);
        
        let identity_box = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), UniqueSdfClassName::new("id_box".to_string()));
        registrator.add(&identity_box);

        let identity_box_frame = NamedSdf::new(SdfBoxFrame::new(Vector::new(1.0, 1.0, 1.0), 0.1), UniqueSdfClassName::new("id_box_frame".to_string()));
        registrator.add(&identity_box_frame);
        
        let xz_torus = {
            let major_radius = 2.0;
            let minor_radius = 1.0;
            NamedSdf::new(SdfTorusXz::new(major_radius, minor_radius), UniqueSdfClassName::new("torus".to_string()))
        };
        registrator.add(&xz_torus);
        
        let capped_xy_torus = {
            let major_radius = 2.0;
            let minor_radius = 1.0;
            let cut_angle = Deg(110.0);
            NamedSdf::new(SdfCappedTorusXy::new(cut_angle, major_radius, minor_radius), UniqueSdfClassName::new("capped_torus".to_string()))
        };
        registrator.add(&capped_xy_torus);
        
        let round_box = NamedSdf::new(SdfRoundBox::new(Vector::new(1.5, 1.0, 1.0), 0.4), UniqueSdfClassName::new("round_box".to_string()));
        registrator.add(&round_box);
        
        let link = NamedSdf::new(SdfLink::new(1.0, 0.5, 0.3), UniqueSdfClassName::new("link".to_string()));
        registrator.add(&link);
        
        let cone = NamedSdf::new(SdfCone::new(Deg(45.0), 1.0), UniqueSdfClassName::new("cone".to_string()));
        registrator.add(&cone);
        
        let hex_prism = NamedSdf::new(SdfHexPrism::new(1.0, 1.0), UniqueSdfClassName::new("hex_prism".to_string()));
        registrator.add(&hex_prism);
        
        let triangular_prism = NamedSdf::new(SdfTriangularPrism::new(1.0, 1.0), UniqueSdfClassName::new("tri_prism".to_string()));
        registrator.add(&triangular_prism);
        
        let capsule = NamedSdf::new(SdfCapsule::new(Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0), 1.0), UniqueSdfClassName::new("capsule".to_string()));
        registrator.add(&capsule);
        
        let cylinder_cross = NamedSdf::new(
            SdfUnion::new(
                SdfCappedCylinderAlongAxis::new(Axis::X, 1.0, 0.2),
                SdfUnion::new(
                    SdfCappedCylinderAlongAxis::new(Axis::Y, 1.0, 0.2),
                    SdfCappedCylinderAlongAxis::new(Axis::Z, 1.0, 0.2),
                ),
            ),
            UniqueSdfClassName::new("cylinder_cross".to_string())
        );
        registrator.add(&cylinder_cross);
        
        let solid_angle = NamedSdf::new(SdfSolidAngle::new(Deg(45.0), 1.0), UniqueSdfClassName::new("solid_angle".to_string()));
        registrator.add(&solid_angle);
        
        let cut_hollow_sphere = NamedSdf::new(SdfCutHollowSphere::new(1.0, 0.3, 0.05), UniqueSdfClassName::new("cut_hollow_sphere".to_string()));
        registrator.add(&cut_hollow_sphere);
        
        let round_cone = NamedSdf::new(SdfRoundCone::new(0.5, 0.1, 1.0), UniqueSdfClassName::new("round_cone".to_string()));
        registrator.add(&round_cone);
        
        let vesica_segment = NamedSdf::new(SdfVesicaSegment::new(0.3, Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0)), UniqueSdfClassName::new("vesica_segment".to_string()));
        registrator.add(&vesica_segment);
        
        let octahedron = NamedSdf::new(SdfOctahedron::new(1.0), UniqueSdfClassName::new("octahedron".to_string()));
        registrator.add(&octahedron);
        
        let rhombus = NamedSdf::new(SdfRhombus::new(1.0, 1.0, 0.1, 0.1), UniqueSdfClassName::new("rhombus".to_string()));
        registrator.add(&rhombus);
        
        let pyramid = NamedSdf::new(SdfPyramid::new(1.0), UniqueSdfClassName::new("pyramid".to_string()));
        registrator.add(&pyramid);
        
        let csg_example_sdf =
            SdfSubtraction::new(
                SdfIntersection::new(
                    SdfSphere::new(1.3),
                    SdfBox::new(Vector::new(1.0, 1.0, 1.0)),
                )
                ,
                SdfUnion::new(
                    SdfCappedCylinderAlongAxis::new(Axis::X, 2.0, 0.4),
                    SdfUnion::new(
                        SdfCappedCylinderAlongAxis::new(Axis::Y, 2.0, 0.4),
                        SdfCappedCylinderAlongAxis::new(Axis::Z, 2.0, 0.4),
                    ),
                ),
            );
        let csg_example = NamedSdf::new(csg_example_sdf, UniqueSdfClassName::new("csg_example".to_string()));
        registrator.add(&csg_example);
        
        let union_smooth = NamedSdf::new(SdfUnionSmooth::new(
            SdfTranslation::new(Vector::new(-0.9, 0.0, 0.0), SdfSphere::new(1.0)),
            SdfTranslation::new(Vector::new( 0.9, 0.0, 0.0), SdfSphere::new(1.0)),
            0.25,
        ), UniqueSdfClassName::new("union_smooth".to_string()));
        registrator.add(&union_smooth);
        
        let subtraction_smooth = NamedSdf::new(
            SdfSubtractionSmooth::new(
                SdfSubtractionSmooth::new(
                    SdfBox::new(Vector::new(1.0, 1.0, 1.0)),
                    SdfTranslation::new(Vector::new( 0.9, 0.0, 0.0), SdfSphere::new(0.5)),
                    0.25,
                ),
                SdfTranslation::new(Vector::new( -0.9, 0.0, 0.0), SdfSphere::new(0.5)),
            0.25,
            ),
            UniqueSdfClassName::new("subtraction_smooth".to_string())
        );
        registrator.add(&subtraction_smooth);
        
        let intersection_smooth = NamedSdf::new(SdfIntersectionSmooth::new(
            SdfTranslation::new(Vector::new(-0.2, 0.0, 0.0), SdfSphere::new(1.0)),
            SdfTranslation::new(Vector::new( 0.2, 0.0, 0.0), SdfSphere::new(1.0)),
            0.25,
        ), UniqueSdfClassName::new("intersection_smooth".to_string()));
        registrator.add(&intersection_smooth);

        let box_to_morph = SdfBox::new(Vector::new(0.2, 0.05, 0.01));

        let twist_time_scale = 1.0;
        let twist_amplitude_scale = 4.0;
        let twisted_box = NamedSdf::new(
            SdfTwisterAlongAxis::new(
                box_to_morph.clone(), Axis::X, twist_time_scale, twist_amplitude_scale,
            ),
            UniqueSdfClassName::new("twisted_box".to_string())
        );
        registrator.add(&twisted_box);

        let bend_time_scale = 1.0;
        let bend_amplitude_scale = 3.0;
        let bent_box = NamedSdf::new(
            SdfBenderAlongAxis::new(
                box_to_morph.clone(), Axis::Y, Axis::X, bend_time_scale, bend_amplitude_scale,
            ),
            UniqueSdfClassName::new("bent_box".to_string())
        );
        registrator.add(&bent_box);

        Self { 
            rectangular_box, 
            sphere,
            quad_0_3,
            identity_box, 
            identity_box_frame, 
            xz_torus, 
            capped_xy_torus, 
            round_box, 
            link, 
            cone, 
            hex_prism,
            triangular_prism,
            capsule,
            cylinder_cross,
            solid_angle,
            cut_hollow_sphere,
            round_cone,
            vesica_segment,
            octahedron,
            rhombus,
            pyramid,
            csg_example,
            union_smooth,
            subtraction_smooth,
            intersection_smooth,
            twisted_box,
            bent_box,
        }
    }
}

pub(super) struct TechWorldMaterials {
    gold_metal: MaterialIndex,
    blue_glass: MaterialIndex,
    purple_glass: MaterialIndex,
    red_glass: MaterialIndex,
    green_mirror: MaterialIndex,
    light: MaterialIndex,
    black: MaterialIndex,
    coral: MaterialIndex,
    red: MaterialIndex,
    blue: MaterialIndex,
    bright_red: MaterialIndex,
    silver: MaterialIndex,
    green: MaterialIndex,
    selected_object: MaterialIndex,
    large_box_material: MaterialIndex,
    deformed_circles_material: MaterialIndex,
    white_chrome_mirror: MaterialIndex,
    large_grid_above_white: MaterialIndex,
    huly_icon_above_blue: MaterialIndex,
    huly_icon_2_vertically_repeated: MaterialIndex,
    rect_grid_bitmap_above_yellow: MaterialIndex,
    checkerboard_small: MaterialIndex,
}

impl TechWorldMaterials {
    #[must_use]
    pub(super) fn new(scene: &mut VisualObjects, procedural_textures: TechWorldProceduralTextures, bitmap_textures: TechWorldBitmapTextures) -> Self {

        let large_grid_above_white = {
            let mut properties = MaterialProperties::new().with_albedo(3.0, 3.0, 3.0);
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(1.0, 0.0, 0.0, 0.5))
                .local_position_to_texture_v(Vector4::new(0.0, -1.0, 0.0, 0.5))
                .wrap_mode([WrapMode::Repeat, WrapMode::Discard])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(bitmap_textures.bitmap_checkerboard_large, builder, &mut properties)
                .expect("failed to make 'bitmap_checkerboard_large' atlas page mapping");
            scene.materials_mutable().deref_mut().add(&properties)
        };

        let huly_icon_above_blue = {
            let mut properties = MaterialProperties::new().with_albedo(0.0, 0.5, 3.0);
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(3.0, 0.0, 0.0, 0.5))
                .local_position_to_texture_v(Vector4::new(0.0, -3.0, 0.0, 0.5))
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(bitmap_textures.bitmap_huly, builder, &mut properties)
                .expect("failed to make 'bitmap_huly' atlas page mapping");
            scene.materials_mutable().deref_mut().add(&properties)
        };

        let huly_icon_2_vertically_repeated = {
            let mut properties = MaterialProperties::new().with_albedo(2.0, 0.5, 0.0);
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(2.0, 0.0, 0.0, 0.5))
                .local_position_to_texture_v(Vector4::new(0.0, -2.0, 0.0, 0.5))
                .wrap_mode([WrapMode::Discard, WrapMode::Repeat])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(bitmap_textures.bitmap_huly_2, builder, &mut properties)
                .expect("failed to make 'bitmap_huly_2' atlas page mapping");
            scene.materials_mutable().deref_mut().add(&properties)
        };

        let rect_grid_bitmap_above_yellow = {
            let mut properties = MaterialProperties::new().with_albedo(1.0, 0.84, 0.0);
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(1.0, 0.0, 0.0, 0.0))
                .local_position_to_texture_v(Vector4::new(0.0, 1.0, 0.0, 0.0))
                .wrap_mode([WrapMode::Clamp, WrapMode::Clamp])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(bitmap_textures.bitmap_rect_grid, builder, &mut properties)
                .expect("failed to make 'bitmap_rect_grid' atlas page mapping");
            scene.materials_mutable().deref_mut().add(&properties)
        };

        let checkerboard_small = {
            let mut properties = MaterialProperties::new().with_albedo(1.0, 1.0, 1.0);
            let builder = AtlasRegionMappingBuilder::new()
                .local_position_to_texture_u(Vector4::new(0.0, 4.0, 0.0, 0.0))
                .local_position_to_texture_v(Vector4::new(0.0, 0.0, 4.0, 0.0))
                .wrap_mode([WrapMode::Repeat, WrapMode::Repeat])
                ;
            scene.mutable_texture_atlas_page_composer()
                .map_into(bitmap_textures.bitmap_checkerboard_small, builder, &mut properties)
                .expect("failed to make 'bitmap_checkerboard_small' atlas page mapping");
            scene.materials_mutable().deref_mut().add(&properties)
        };

        let materials = scene.materials_mutable();

        let gold_metal = materials.add(
            &MaterialProperties::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(1.0, 0.5, 0.0)
                .with_specular_strength(0.00001)
                .with_roughness(0.25)
                .with_refractive_index_eta(0.0)
        );

        let large_box_material = materials.add(&MaterialProperties::new()
            .with_albedo(0.95, 0.95, 0.95)
            .with_refractive_index_eta(2.5));

        let blue_glass = materials
            .add(&MaterialProperties::new().with_class(MaterialClass::Glass).with_albedo(0.0, 0.5, 0.9).with_refractive_index_eta(1.4));

        let purple_glass = materials
            .add(&MaterialProperties::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.5, 0.9).with_refractive_index_eta(1.01));

        let red_glass = materials
            .add(&MaterialProperties::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.2, 0.0).with_refractive_index_eta(1.4));

        let green_mirror = materials.add(
            &MaterialProperties::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.64, 0.77, 0.22)
                .with_refractive_index_eta(1.4)
        );

        let light = materials.add(
            &MaterialProperties::new()
                .with_emission(2.0, 2.0, 2.0)
        );

        let black = materials.add(
            &MaterialProperties::new()
                .with_albedo(0.2, 0.2, 0.2)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let coral = materials.add(
            &MaterialProperties::new()
                .with_albedo(1.0, 0.5, 0.3)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.01)
                .with_roughness(0.0)
                .with_albedo_texture(TextureReference::Procedural(procedural_textures.lava()))
        );

        let deformed_circles_material = materials.add(
            &MaterialProperties::new()
                .with_albedo(1.0, 1.0, 1.0)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.01)
                .with_roughness(0.0)
                .with_albedo_texture(TextureReference::Procedural(procedural_textures.deformed_circles()))
        );

        let red = materials.add(
            &MaterialProperties::new()
                .with_albedo(0.75, 0.1, 0.1)
                .with_specular(0.75, 0.1, 0.1)
                .with_specular_strength(0.05)
                .with_roughness(0.95)
        );

        let blue = materials.add(
            &MaterialProperties::new()
                .with_albedo(0.1, 0.1, 0.75)
                .with_specular(0.1, 0.1, 0.75)
                .with_specular_strength(0.05)
                .with_roughness(0.95)
                .with_albedo_texture(TextureReference::Procedural(procedural_textures.water()))
        );

        let bright_red = materials.add(
            &MaterialProperties::new()
                .with_albedo(1.0, 0.0, 0.0)
                .with_specular(1.0, 1.0, 1.0)
                .with_specular_strength(0.05)
                .with_roughness(0.95)
        );

        let silver = materials.add(
            &MaterialProperties::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.75, 0.75, 0.75)
                .with_specular(0.75, 0.75, 0.75)
                .with_specular_strength(0.55)
                .with_roughness(0.0)
        );

        let green = materials.add(
            &MaterialProperties::new()
                .with_albedo(0.05, 0.55, 0.05)
                .with_specular(0.05, 0.55, 0.05)
                .with_specular_strength(0.05)
                .with_roughness(0.95)
        );

        let selected_object = materials.add(
            &MaterialProperties::new()
                .with_albedo(0.05, 0.05, 2.05)
                .with_specular(0.05, 0.05, 0.55)
                .with_specular_strength(0.15)
                .with_roughness(0.45)
                .with_albedo_texture(TextureReference::Procedural(procedural_textures.checkerboard()))
        );

        let white_chrome_mirror = materials.add(
            &MaterialProperties::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(1.0, 1.0, 1.0)
                .with_specular(0.0, 0.0, 0.0)
                .with_specular_strength(0.0)
                .with_roughness(0.0)
        );

        Self {
            gold_metal,
            blue_glass,
            purple_glass,
            red_glass,
            green_mirror,
            light,
            black,
            coral,
            red,
            blue,
            bright_red,
            silver,
            green,
            selected_object,
            large_box_material,
            deformed_circles_material,
            white_chrome_mirror,
            large_grid_above_white,
            huly_icon_above_blue,
            huly_icon_2_vertically_repeated,
            rect_grid_bitmap_above_yellow,
            checkerboard_small,
        }
    }
}

pub(super) struct TechWorldProceduralTextures {
    checkerboard: ProceduralTextureUid,
    lava: ProceduralTextureUid,
    water: ProceduralTextureUid,
    deformed_circles: ProceduralTextureUid,
}

impl TechWorldProceduralTextures {
    #[must_use]
    pub(super) fn new(container: &mut ProceduralTextures) -> Self {
        let checkerboard = container.add(make_checkerboard_texture(10.0), None);

        let mut triplanar_mapper = container.make_triplanar_mapper();

        let lava_texture_2d = Self::make_lava_like_texture();
        let lava_texture_3d = triplanar_mapper.make_triplanar_mapping(&lava_texture_2d, 8.0, None);
        let lava = container.add(lava_texture_3d, None);

        let water_texture_2d = Self::make_water_like_texture();
        let water_texture_3d = triplanar_mapper.make_triplanar_mapping(&water_texture_2d, 8.0, None);
        let water = container.add(water_texture_3d, None);

        let deformed_circles_texture_2d = Self::make_deformed_circles_texture();
        let deformed_circles_texture_3d = triplanar_mapper.make_triplanar_mapping(&deformed_circles_texture_2d, 8.0, None);
        let deformed_circles = container.add(deformed_circles_texture_3d, None);

        Self { checkerboard, lava, water, deformed_circles }
    }

    #[must_use]
    fn make_lava_like_texture() -> TextureProcedural2D {
        Self::make_texture_2d(include_str!("texture_2d_lava_like.wgsl"), "lava_like_texture")
    }

    #[must_use]
    fn make_water_like_texture() -> TextureProcedural2D {
        Self::make_texture_2d(include_str!("texture_2d_water_like.wgsl"), "water_like_surface")
    }

    #[must_use]
    fn make_deformed_circles_texture() -> TextureProcedural2D {
        Self::make_texture_2d(include_str!("texture_2d_filtered_deformed_circles.wgsl"), "deformed_circles_texture")
    }

    #[must_use]
    fn make_texture_2d(utilities: &str, main_function: &str) -> TextureProcedural2D {
        let body = format!(
            "return {main_function}({uv_parameter_name}, {time_parameter}, {dp_dx_parameter}, {dp_dy_parameter});",
            uv_parameter_name=conventions::PARAMETER_NAME_2D_TEXTURE_COORDINATES,
            time_parameter=conventions::PARAMETER_NAME_THE_TIME,
            dp_dx_parameter = conventions::PARAMETER_DP_DX,
            dp_dy_parameter = conventions::PARAMETER_DP_DY,
        );
        TextureProcedural2D::new(ShaderCode::<Generic>::new(utilities.to_string()), ShaderCode::<FunctionBody>::new(body.to_string()))
    }

    #[must_use]
    fn checkerboard(&self) -> ProceduralTextureUid {
        self.checkerboard
    }

    #[must_use]
    fn lava(&self) -> ProceduralTextureUid {
        self.lava
    }

    #[must_use]
    fn water(&self) -> ProceduralTextureUid {
        self.water
    }

    #[must_use]
    fn deformed_circles(&self) -> ProceduralTextureUid {
        self.deformed_circles
    }
}

pub(super) struct TechWorld {
    sdf_classes: TechWorldSdfClasses,
    materials: TechWorldMaterials,
    
    light_panel: Option<ObjectUid>,
    light_panel_z: f64,
    light_panel_x: f64,

    infinitely_twisted_button: Option<ObjectUid>,
    single_twisted_button: Option<ObjectUid>,
    back_n_forth_twisted_button: Option<ObjectUid>,
    very_slow_twisted_button: Option<ObjectUid>,

    infinitely_bent_button: Option<ObjectUid>,
    single_bent_button: Option<ObjectUid>,
    back_n_forth_bent_button: Option<ObjectUid>,
    very_slow_bent_button: Option<ObjectUid>,
}

impl TechWorld {
    #[must_use]
    pub(super) fn new(sdf_classes: TechWorldSdfClasses, materials: TechWorldMaterials) -> Self {
        Self { 
            sdf_classes,
            materials,

            light_panel: None,
            light_panel_z: -1.0,
            light_panel_x: -1.0,

            infinitely_twisted_button: None,
            single_twisted_button: None,
            back_n_forth_twisted_button: None,
            very_slow_twisted_button: None,

            infinitely_bent_button: None,
            single_bent_button: None,
            back_n_forth_bent_button: None,
            very_slow_bent_button: None,
        }
    }
    
    #[must_use]
    pub(super) fn selected_object_material(&self) -> MaterialIndex {
        self.materials.selected_object
    }

    pub(super) fn move_light_z(&mut self, sign: f64, scene: &mut Hub) {
        self.light_panel_z += sign * 0.01;
        self.invalidate_light_panel(scene);
    }
    
    pub(super) fn move_light_x(&mut self, sign: f64, scene: &mut Hub) {
        self.light_panel_x += sign * 0.01;
        self.invalidate_light_panel(scene);
    }

    fn invalidate_light_panel(&mut self, scene: &mut Hub) {
        if let Some(light_panel) = self.light_panel {
            scene.delete(light_panel);
        }
        self.make_light_panel(scene);
    }

    fn make_light_panel(&mut self, scene: &mut Hub) {
        self.light_panel = Some(
            scene.add_parallelogram(Point::new(self.light_panel_x, 1.0, self.light_panel_z), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), self.materials.light)
        );
    }

    fn make_common_scene_walls(&mut self, scene: &mut Hub) {
        self.make_light_panel(scene);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), self.materials.black);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), self.materials.red);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), self.materials.green);
    }

    fn clear_scene(&mut self, scene: &mut Hub) {
        scene.clear_objects();
        
        self.infinitely_twisted_button = None;
        self.single_twisted_button = None;
        self.back_n_forth_twisted_button = None;
        self.very_slow_twisted_button = None;

        self.infinitely_bent_button = None;
        self.single_bent_button = None;
        self.back_n_forth_bent_button = None;
        self.very_slow_bent_button = None;
        
        self.light_panel = None;
    }

    pub(super) fn load_smooth_operators_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);

        self.make_common_scene_walls(scene);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.0, -0.8))*Affine::from_scale(0.2)),
            self.sdf_classes.union_smooth.name(),
            self.materials.blue);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.9, 0.0, -0.8))*Affine::from_scale(0.2)),
            self.sdf_classes.subtraction_smooth.name(),
            self.materials.deformed_circles_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, -0.5, -0.8))*Affine::from_scale(0.2)),
            self.sdf_classes.intersection_smooth.name(),
            self.materials.green_mirror);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.4, -0.3))*Affine::from_scale(0.3)),
            self.sdf_classes.csg_example.name(),
            self.materials.blue);
    }

    pub(super) fn load_sdf_exhibition_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);

        self.make_common_scene_walls(scene);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.8, -0.9))*Affine::from_scale(0.1)),
            self.sdf_classes.sphere.name(),
            self.materials.green);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.75, -0.9))*Affine::from_nonuniform_scale(0.1, 0.2, 0.01)),
            self.sdf_classes.identity_box.name(),
            self.materials.blue);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.2, 0.75, -0.9))*Affine::from_scale(0.1)),
            self.sdf_classes.identity_box_frame.name(),
            self.materials.red);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.15, 0.75, -0.9))*Affine::from_nonuniform_scale(0.05, 0.05, 0.05)*Affine::from_angle_x(Deg(90.0))),
            self.sdf_classes.xz_torus.name(),
            self.materials.gold_metal);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.55, 0.7, -0.9))*Affine::from_nonuniform_scale(0.1, 0.1, 0.08)*Affine::from_angle_x(Deg(90.0))),
            self.sdf_classes.round_box.name(),
            self.materials.blue_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.95, 0.75, -0.85))*Affine::from_scale(0.05)),
            self.sdf_classes.capped_xy_torus.name(),
            self.materials.coral);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.3, 0.75, -0.95))*Affine::from_scale(0.1)),
            self.sdf_classes.link.name(),
            self.materials.purple_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.7, 0.8, -0.8))*Affine::from_scale(0.2)),
            self.sdf_classes.cone.name(),
            self.materials.silver);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.4, -0.8))*Affine::from_nonuniform_scale(0.1, 0.1, 0.05)),
            self.sdf_classes.hex_prism.name(),
            self.materials.bright_red);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.35, -0.8))*Affine::from_nonuniform_scale(0.1, 0.1, 0.05)),
            self.sdf_classes.triangular_prism.name(),
            self.materials.coral);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.2, 0.35, -0.8))*Affine::from_angle_z(Deg(90.0))*Affine::from_nonuniform_scale(0.1, 0.05, 0.05)),
            self.sdf_classes.capsule.name(),
            self.materials.green);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.1, 0.35, -0.8))*Affine::from_scale(0.1)),
            self.sdf_classes.cylinder_cross.name(),
            self.materials.blue);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.4, 0.35, -0.9))*Affine::from_scale(0.1)),
            self.sdf_classes.solid_angle.name(),
            self.materials.red);
        
        scene.add_sdf(
            &(
                Affine::from_translation(Vector::new(0.7, 0.35, -0.9))*
                Affine::from_angle_x(Deg(45.0))*
                Affine::from_angle_z(Deg(15.0))*
                Affine::from_scale(0.1)),
            self.sdf_classes.cut_hollow_sphere.name(),
            self.materials.bright_red);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.0, 0.35, -0.9))*Affine::from_scale(0.15)),
            self.sdf_classes.round_cone.name(),
            self.materials.black);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.3, 0.35, -0.9))*Affine::from_scale(0.15)),
            self.sdf_classes.vesica_segment.name(),
            self.materials.gold_metal);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.7, 0.35, -0.85))*Affine::from_scale(0.15)),
            self.sdf_classes.octahedron.name(),
            self.materials.purple_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.0, -0.9))*Affine::from_angle_x(Deg(90.0))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.rhombus.name(),
            self.materials.red_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.0, -0.9))*Affine::from_scale(0.15)),
            self.sdf_classes.pyramid.name(),
            self.materials.blue);
    }

    pub(super) fn load_triangle_mesh_testing_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);

        self.make_common_scene_walls(scene);

        let mut meshes = MeshWarehouse::new();
        let mesh_file = get_resource_path(Path::new(CONTENT_ROOT_FOLDER_NAME).join("monkey.obj"));
        let mesh_or_error = meshes.load(mesh_file);

        match mesh_or_error {
            Ok(mesh) => {
                let location 
                    = Transformation::new(
                    Affine::from_translation(Vector::new(0.5, 0.0, 0.0)) *
                        Affine::from_scale(1.0)
                    );
                scene.add_mesh(&meshes, mesh, &location, self.materials.black);
            },
            Err(mesh_loading_error) => {
                error!("failed to load mesh: {mesh_loading_error}");
            },
        }
    }

    pub(super) fn load_morphing_demo_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);
        
        scene.add_parallelogram(Point::new(-0.15, -0.15, 2.0), Vector::new(0.3, 0.0, 0.0), Vector::new(0.0, 0.3, 0.0), self.materials.light);
        
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), self.materials.black);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), self.materials.red);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), self.materials.green);

        // twist demo

        const TWIST_RAY_MARCH_FIX: f64 = 0.9;

        self.infinitely_twisted_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.2, 0.3, 0.0)),
            TWIST_RAY_MARCH_FIX,
            self.sdf_classes.twisted_box.name(),
            self.materials.red_glass));

        self.single_twisted_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.2, 0.1, 0.0)),
            TWIST_RAY_MARCH_FIX,
            self.sdf_classes.twisted_box.name(),
            self.materials.gold_metal));

        self.back_n_forth_twisted_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.2, -0.1, 0.0)),
            TWIST_RAY_MARCH_FIX,
            self.sdf_classes.twisted_box.name(),
            self.materials.blue));

        self.very_slow_twisted_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.2, -0.3, 0.0)),
            TWIST_RAY_MARCH_FIX,
            self.sdf_classes.twisted_box.name(),
            self.materials.green_mirror));

        // bend demo

        const BEND_RAY_MARCH_FIX: f64 = 0.7;

        self.infinitely_bent_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.7, 0.3, 0.0)),
            BEND_RAY_MARCH_FIX,
            self.sdf_classes.bent_box.name(),
            self.materials.green));

        self.single_bent_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.7, 0.1, 0.0)),
            BEND_RAY_MARCH_FIX,
            self.sdf_classes.bent_box.name(),
            self.materials.coral));

        self.back_n_forth_bent_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.7, -0.1, 0.0)),
            BEND_RAY_MARCH_FIX,
            self.sdf_classes.bent_box.name(),
            self.materials.purple_glass));

        self.very_slow_bent_button = Some(scene.add_sdf_with_ray_march_fix(
            &Affine::from_translation(Vector::new(0.7, -0.3, 0.0)),
            BEND_RAY_MARCH_FIX,
            self.sdf_classes.bent_box.name(),
            self.materials.red));
    }

    pub(super) fn load_ui_box_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.7, 0.2, -0.7))*Affine::from_angle_z(Deg(-30.0))),
            self.sdf_classes.rectangular_box.name(),
            self.materials.silver);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.5, -0.4, -0.9))*Affine::from_angle_z(Deg(30.0))),
            self.sdf_classes.rectangular_box.name(),
            self.materials.bright_red);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.5, 0.6, -1.0))*Affine::from_scale(0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.gold_metal);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.5, 0.0, 2.0))*Affine::from_scale(0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.blue_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.5, 0.0, -0.88))*Affine::from_scale(0.1) ),
            self.sdf_classes.sphere.name(),
            self.materials.blue_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.5, 0.0, -1.0))*Affine::from_nonuniform_scale(0.25, 0.1, 0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.green_mirror);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.0, -1.0))*Affine::from_scale(0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.coral);

        self.make_common_scene_walls(scene);

        let mut meshes = MeshWarehouse::new();
        let cube_mesh_file = get_resource_path(Path::new(CONTENT_ROOT_FOLDER_NAME).join("cube.obj"));
        let cube_mesh_or_error = meshes.load(cube_mesh_file);
        
        match cube_mesh_or_error {
            Ok(cube_mesh) => {
                let large_box_location =
                    Transformation::new(
                        Affine::from_translation(Vector::new(0.15, 0.6, -1.0)) *
                            Affine::from_nonuniform_scale(3.65, 0.8, 0.25));
                scene.add_mesh(&meshes, cube_mesh, &large_box_location, self.materials.large_box_material);
        
                {
                    let box_location =Transformation::new(
                        Affine::from_translation(Vector::new(-0.4, 0.1, -1.0)) * Affine::from_scale(0.4));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, self.materials.gold_metal);
                }
        
                {
                    let box_location = Transformation::new(
                        Affine::from_translation(Vector::new(0.9, -0.4, -1.0)) * Affine::from_scale(0.4));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, self.materials.purple_glass);
                }
        
                {
                    let box_location = Transformation::new(
                        Affine::from_translation(Vector::new(0.4, 0.1, 0.2)) * Affine::from_nonuniform_scale(0.9, 0.9, 0.1));
                    scene.add_mesh(&meshes, cube_mesh, &box_location, self.materials.red_glass);
                }
            },
            Err(mesh_loading_error) => {
                error!("failed to load cube mesh: {mesh_loading_error}");
            },
        }
    }

    pub(super) fn load_bitmap_texturing_demo_scene(&mut self, scene: &mut Hub) {
        self.clear_scene(scene);

        self.make_common_scene_walls(scene);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.4, 0.0, -0.45)) * Affine::from_nonuniform_scale(0.5, 0.5, 0.5)),
            self.sdf_classes.sphere.name(),
            self.materials.white_chrome_mirror);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.4, 0.0, -0.45)) * Affine::from_nonuniform_scale(0.5, 0.5, 0.5)),
            self.sdf_classes.sphere.name(),
            self.materials.purple_glass);

        scene.add_parallelogram(
            Point::new(0.2, -0.6, -0.7),
            Vector::new(0.3, 0.0, 0.0),
            Vector::new(0.0, 0.3, 0.0),
            self.materials.huly_icon_above_blue
        );

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.05, -0.45, -0.7))),
            self.sdf_classes.quad_0_3.name(),
            self.materials.huly_icon_above_blue);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.7, -0.45, -0.7)) * Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.sphere.name(),
            self.materials.large_grid_above_white
        );

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.35, -0.1, -0.7)) * Affine::from_nonuniform_scale(0.1, 0.1, 0.02)),
            self.sdf_classes.hex_prism.name(),
            self.materials.rect_grid_bitmap_above_yellow
        );

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.7, -0.1, -0.7)) * Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.sphere.name(),
            self.materials.checkerboard_small
        );

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.7, 0.1, -0.7)) * Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.round_cone.name(),
            self.materials.huly_icon_2_vertically_repeated
        );
    }

    #[must_use]
    pub(super) fn infinitely_twisted_button(&self) -> Option<ObjectUid> {
        self.infinitely_twisted_button
    }
    #[must_use]
    pub(super) fn single_twisted_button(&self) -> Option<ObjectUid> {
        self.single_twisted_button
    }
    #[must_use]
    pub(super) fn back_n_forth_twisted_button(&self) -> Option<ObjectUid> {
        self.back_n_forth_twisted_button
    }
    #[must_use]
    pub(super) fn very_slow_twisted_button(&self) -> Option<ObjectUid> {
        self.very_slow_twisted_button
    }

    #[must_use]
    pub(super) fn infinitely_bent_button(&self) -> Option<ObjectUid> {
        self.infinitely_bent_button
    }
    #[must_use]
    pub(super) fn single_bent_button(&self) -> Option<ObjectUid> {
        self.single_bent_button
    }
    #[must_use]
    pub(super) fn back_n_forth_bent_button(&self) -> Option<ObjectUid> {
        self.back_n_forth_bent_button
    }
    #[must_use]
    pub(super) fn very_slow_bent_button(&self) -> Option<ObjectUid> {
        self.very_slow_bent_button
    }
}

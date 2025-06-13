use cgmath::Deg;
use library::geometry::alias::{Point, Vector};
use library::geometry::axis::Axis;
use library::geometry::transform::{Affine, Transformation};
use library::objects::material::{Material, MaterialClass};
use library::objects::material_index::MaterialIndex;
use library::sdf::code_generator::SdfRegistrator;
use library::sdf::named_sdf::{NamedSdf, UniqueSdfClassName};
use library::sdf::sdf_box::SdfBox;
use library::sdf::sdf_box_frame::SdfBoxFrame;
use library::sdf::sdf_capped_cylinder_along_axis::SdfCappedCylinderAlongAxis;
use library::sdf::sdf_capped_torus_xy::SdfCappedTorusXy;
use library::sdf::sdf_capsule::SdfCapsule;
use library::sdf::sdf_cone::SdfCone;
use library::sdf::sdf_cut_hollow_sphere::SdfCutHollowSphere;
use library::sdf::sdf_hex_prism::SdfHexPrism;
use library::sdf::sdf_intersection::SdfIntersection;
use library::sdf::sdf_intersection_smooth::SdfIntersectionSmooth;
use library::sdf::sdf_link::SdfLink;
use library::sdf::sdf_octahedron::SdfOctahedron;
use library::sdf::sdf_pyramid::SdfPyramid;
use library::sdf::sdf_rhombus::SdfRhombus;
use library::sdf::sdf_round_box::SdfRoundBox;
use library::sdf::sdf_round_cone::SdfRoundCone;
use library::sdf::sdf_solid_angle::SdfSolidAngle;
use library::sdf::sdf_sphere::SdfSphere;
use library::sdf::sdf_subtraction::SdfSubtraction;
use library::sdf::sdf_subtraction_smooth::SdfSubtractionSmooth;
use library::sdf::sdf_torus_xz::SdfTorusXz;
use library::sdf::sdf_translation::SdfTranslation;
use library::sdf::sdf_triangular_prism::SdfTriangularPrism;
use library::sdf::sdf_union::SdfUnion;
use library::sdf::sdf_union_smooth::SdfUnionSmooth;
use library::sdf::sdf_vesica_segment::SdfVesicaSegment;
use library::utils::object_uid::ObjectUid;
use log::error;
use std::env;
use std::path::{Path, PathBuf};
use library::container::container::Container;
use library::container::mesh_warehouse::MeshWarehouse;
use library::scene::scene::Scene;

pub(super) struct SdfClasses {
    rectangular_box: NamedSdf,
    sphere: NamedSdf,
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
}

impl SdfClasses {
    #[must_use]
    pub(super) fn new(registrator: &mut SdfRegistrator) -> Self {
        let rectangular_box = NamedSdf::new(SdfBox::new(Vector::new(0.24, 0.1, 0.24)), UniqueSdfClassName::new("button".to_string()));
        registrator.add(&rectangular_box);

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

        Self { 
            rectangular_box, 
            sphere, 
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
        }
    }
}

pub(super) struct Materials {
    gold_metal: MaterialIndex,
    blue_glass: MaterialIndex,
    purple_glass: MaterialIndex,
    red_glass: MaterialIndex,
    green_mirror: MaterialIndex,
    light_material: MaterialIndex,
    black_material: MaterialIndex,
    coral_material: MaterialIndex,
    red_material: MaterialIndex,
    blue_material: MaterialIndex,
    bright_red_material: MaterialIndex,
    silver_material: MaterialIndex,
    green_material: MaterialIndex,
    selected_object_material: MaterialIndex,
    large_box_material: MaterialIndex,
}

impl Materials {
    #[must_use]
    pub(super) fn new(scene: &mut Container) -> Self {
        let materials = scene.materials_mutable();
        
        let gold_metal = materials.add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(1.0, 0.5, 0.0)
                .with_specular_strength(0.00001)
                .with_roughness(0.25)
                .with_refractive_index_eta(0.0),
        );

        let large_box_material = materials.add(&Material::new()
            .with_albedo(0.95, 0.95, 0.95)
            .with_refractive_index_eta(2.5));

        let blue_glass = materials
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(0.0, 0.5, 0.9).with_refractive_index_eta(1.4));

        let purple_glass = materials
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.5, 0.9).with_refractive_index_eta(1.4));

        let red_glass = materials
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.2, 0.0).with_refractive_index_eta(1.4));

        let green_mirror = materials.add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.64, 0.77, 0.22)
                .with_refractive_index_eta(1.4),
        );

        let light_material = materials.add(
            &Material::new()
                .with_emission(2.0, 2.0, 2.0)
        );

        let black_material = materials.add(
            &Material::new()
                .with_albedo(0.2, 0.2, 0.2)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let coral_material = materials.add(
            &Material::new()
                .with_albedo(1.0, 0.5, 0.3)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.01)
                .with_roughness(0.0),
        );

        let red_material = materials.add(
            &Material::new()
                .with_albedo(0.75, 0.1, 0.1)
                .with_specular(0.75, 0.1, 0.1)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let blue_material = materials.add(
            &Material::new()
                .with_albedo(0.1, 0.1, 0.75)
                .with_specular(0.1, 0.1, 0.75)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let bright_red_material = materials.add(
            &Material::new()
                .with_albedo(1.0, 0.0, 0.0)
                .with_specular(1.0, 1.0, 1.0)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let silver_material = materials.add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.75, 0.75, 0.75)
                .with_specular(0.75, 0.75, 0.75)
                .with_specular_strength(0.55)
                .with_roughness(0.0),
        );

        let green_material = materials.add(
            &Material::new()
                .with_albedo(0.05, 0.55, 0.05)
                .with_specular(0.05, 0.55, 0.05)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let selected_object_material = materials.add(
            &Material::new()
                .with_albedo(0.05, 0.05, 2.05)
                .with_specular(0.05, 0.05, 0.55)
                .with_specular_strength(0.15)
                .with_roughness(0.45),
        );

        Self {
            gold_metal,
            blue_glass,
            purple_glass,
            red_glass,
            green_mirror,
            light_material,
            black_material,
            coral_material,
            red_material,
            blue_material,
            bright_red_material,
            silver_material,
            green_material,
            selected_object_material,
            large_box_material,
        }
    }
}

pub(super) struct TechWorld {
    sdf_classes: SdfClasses,
    materials: Materials,
    
    light_panel: Option<ObjectUid>,
    light_panel_z: f64,
    light_panel_x: f64,
}

impl TechWorld {
    #[must_use]
    pub(super) fn new(sdf_classes: SdfClasses, materials: Materials) -> Self {
        Self { 
            sdf_classes, materials, light_panel: None, light_panel_z: -1.0, light_panel_x: -1.0,
        }
    }

    #[must_use]
    pub(super) fn selected_object_material(&self) -> MaterialIndex {
        self.materials.selected_object_material
    }

    pub(super) fn move_light_z(&mut self, sign: f64, scene: &mut Scene) {
        self.light_panel_z += sign * 0.01;
        self.invalidate_light_panel(scene);
    }
    
    pub(super) fn move_light_x(&mut self, sign: f64, scene: &mut Scene) {
        self.light_panel_x += sign * 0.01;
        self.invalidate_light_panel(scene);
    }

    fn invalidate_light_panel(&mut self, scene: &mut Scene) {
        if let Some(light_panel) = self.light_panel {
            scene.delete(light_panel);
        }
        self.make_light_panel(scene);
    }

    fn make_light_panel(&mut self, scene: &mut Scene) {
        self.light_panel = Some(
            scene.add_parallelogram(Point::new(self.light_panel_x, 1.0, self.light_panel_z), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), self.materials.light_material)
        );
    }

    fn make_common_scene_walls(&mut self, scene: &mut Scene) {
        self.make_light_panel(scene);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), self.materials.black_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), self.materials.red_material);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), self.materials.green_material);
    }

    pub(super) fn load_to_smooth_operators_scene(&mut self, scene: &mut Scene) {
        scene.clear_objects();

        self.make_common_scene_walls(scene);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.0, -0.8))*Affine::from_nonuniform_scale(0.2, 0.2, 0.2)),
            self.sdf_classes.union_smooth.name(),
            self.materials.blue_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.9, 0.0, -0.8))*Affine::from_nonuniform_scale(0.2, 0.2, 0.2)),
            self.sdf_classes.subtraction_smooth.name(),
            self.materials.red_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, -0.5, -0.8))*Affine::from_nonuniform_scale(0.2, 0.2, 0.2)),
            self.sdf_classes.intersection_smooth.name(),
            self.materials.green_mirror);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.4, -0.3))*Affine::from_nonuniform_scale(0.3, 0.3, 0.3)),
            self.sdf_classes.csg_example.name(),
            self.materials.blue_material);
    }

    pub(super) fn load_to_sdf_exhibition_scene(&mut self, scene: &mut Scene) {
        scene.clear_objects();

        self.make_common_scene_walls(scene);

        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.8, -0.9))*Affine::from_scale(0.1)),
            self.sdf_classes.sphere.name(),
            self.materials.green_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.75, -0.9))*Affine::from_nonuniform_scale(0.1, 0.2, 0.01)),
            self.sdf_classes.identity_box.name(),
            self.materials.blue_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.2, 0.75, -0.9))*Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.identity_box_frame.name(),
            self.materials.red_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.15, 0.75, -0.9))*Affine::from_nonuniform_scale(0.05, 0.05, 0.05)*Affine::from_angle_x(Deg(90.0))),
            self.sdf_classes.xz_torus.name(),
            self.materials.gold_metal);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.55, 0.7, -0.9))*Affine::from_nonuniform_scale(0.1, 0.1, 0.08)*Affine::from_angle_x(Deg(90.0))),
            self.sdf_classes.round_box.name(),
            self.materials.blue_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.95, 0.75, -0.85))*Affine::from_nonuniform_scale(0.05, 0.05, 0.05)),
            self.sdf_classes.capped_xy_torus.name(),
            self.materials.coral_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.3, 0.75, -0.95))*Affine::from_scale(0.1)),
            self.sdf_classes.link.name(),
            self.materials.purple_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.7, 0.8, -0.8))*Affine::from_scale(0.2)),
            self.sdf_classes.cone.name(),
            self.materials.silver_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.4, -0.8))*Affine::from_nonuniform_scale(0.1, 0.1, 0.05)),
            self.sdf_classes.hex_prism.name(),
            self.materials.bright_red_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.35, -0.8))*Affine::from_nonuniform_scale(0.1, 0.1, 0.05)),
            self.sdf_classes.triangular_prism.name(),
            self.materials.coral_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.2, 0.35, -0.8))*Affine::from_angle_z(Deg(90.0))*Affine::from_nonuniform_scale(0.1, 0.05, 0.05)),
            self.sdf_classes.capsule.name(),
            self.materials.green_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.1, 0.35, -0.8))*Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.cylinder_cross.name(),
            self.materials.blue_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.4, 0.35, -0.9))*Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.solid_angle.name(),
            self.materials.red_material);
        
        scene.add_sdf(
            &(
                Affine::from_translation(Vector::new(0.7, 0.35, -0.9))*
                Affine::from_angle_x(Deg(45.0))*
                Affine::from_angle_z(Deg(15.0))*
                Affine::from_nonuniform_scale(0.1, 0.1, 0.1)),
            self.sdf_classes.cut_hollow_sphere.name(),
            self.materials.bright_red_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.0, 0.35, -0.9))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.round_cone.name(),
            self.materials.black_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.3, 0.35, -0.9))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.vesica_segment.name(),
            self.materials.gold_metal);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.7, 0.35, -0.85))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.octahedron.name(),
            self.materials.purple_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.8, 0.0, -0.9))*Affine::from_angle_x(Deg(90.0))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.rhombus.name(),
            self.materials.red_glass);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(-0.5, 0.0, -0.9))*Affine::from_nonuniform_scale(0.15, 0.15, 0.15)),
            self.sdf_classes.pyramid.name(),
            self.materials.blue_material);
    }
    
    #[must_use]
    fn get_resource_path(file_name: impl AsRef<Path>) -> PathBuf {
        let exe_path = env::current_exe().unwrap();
        let exe_directory = exe_path.parent().unwrap();
        exe_directory.join(file_name)
    }

    pub(super) fn load_to_triangle_mesh_testing_scene(&mut self, scene: &mut Scene) {
        scene.clear_objects();
        self.make_common_scene_walls(scene);

        let mut meshes = MeshWarehouse::new();
        let mesh_file = Self::get_resource_path(Path::new("assets").join("monkey.obj"));
        let mesh_or_error = meshes.load(mesh_file);

        match mesh_or_error {
            Ok(mesh) => {
                let location 
                    = Transformation::new(
                    Affine::from_translation(Vector::new(0.5, 0.0, 0.0)) *
                        Affine::from_nonuniform_scale(0.6, 0.6, 0.6)
                    );
                scene.add_mesh(&meshes, mesh, &location, self.materials.black_material);
            },
            Err(mesh_loading_error) => {
                error!("failed to load mesh: {}", mesh_loading_error);
            },
        }
    }
    
    pub(super) fn load_to_ui_box_scene(&mut self, scene: &mut Scene) {
        scene.clear_objects();
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.7, 0.2, -0.7))*Affine::from_angle_z(Deg(-30.0))),
            self.sdf_classes.rectangular_box.name(),
            self.materials.silver_material);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(1.5, -0.4, -0.9))*Affine::from_angle_z(Deg(30.0))),
            self.sdf_classes.rectangular_box.name(),
            self.materials.bright_red_material);
        
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
            &(Affine::from_translation(Vector::new(1.5, 0.0, -1.0))*Affine::from_scale(0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.green_mirror);
        
        scene.add_sdf(
            &(Affine::from_translation(Vector::new(0.0, 0.0, -1.0))*Affine::from_scale(0.25) ),
            self.sdf_classes.sphere.name(),
            self.materials.coral_material);

        self.make_common_scene_walls(scene);

        let mut meshes = MeshWarehouse::new();
        let cube_mesh_file = Self::get_resource_path(Path::new("assets").join("cube.obj"));
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
                error!("failed to load cube mesh: {}", mesh_loading_error);
            },
        }
    }
}

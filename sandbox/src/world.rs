use std::env;
use std::path::{Path, PathBuf};
use cgmath::Deg;
use log::error;
use library::geometry::alias::{Point, Vector};
use library::geometry::transform::{Affine, Transformation};
use library::objects::material::{Material, MaterialClass};
use library::objects::material_index::MaterialIndex;
use library::scene::container::Container;
use library::scene::mesh_warehouse::MeshWarehouse;
use library::sdf::code_generator::SdfRegistrator;
use library::sdf::named_sdf::{NamedSdf, UniqueName};
use library::sdf::sdf_box::SdfBox;
use library::sdf::sdf_box_frame::SdfBoxFrame;
use library::sdf::sdf_capped_torus_xy::SdfCappedTorusXy;
use library::sdf::sdf_cone::SdfCone;
use library::sdf::sdf_hex_prism::SdfHexPrism;
use library::sdf::sdf_link::SdfLink;
use library::sdf::sdf_round_box::SdfRoundBox;
use library::sdf::sdf_sphere::SdfSphere;
use library::sdf::sdf_torus_xz::SdfTorusXz;

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
}

impl SdfClasses {
    #[must_use]
    pub(super) fn new(registrator: &mut SdfRegistrator) -> Self {
        let rectangular_box = NamedSdf::new(SdfBox::new(Vector::new(0.24, 0.1, 0.02)), UniqueName::new("button".to_string()));
        registrator.add(&rectangular_box);

        let sphere = NamedSdf::new(SdfSphere::new(1.0), UniqueName::new("bubble".to_string()));
        registrator.add(&sphere);
        
        let identity_box = NamedSdf::new(SdfBox::new(Vector::new(1.0, 1.0, 1.0)), UniqueName::new("id_box".to_string()));
        registrator.add(&identity_box);

        let identity_box_frame = NamedSdf::new(SdfBoxFrame::new(Vector::new(1.0, 1.0, 1.0), 0.1), UniqueName::new("id_box_frame".to_string()));
        registrator.add(&identity_box_frame);
        
        let xz_torus = {
            let major_radius = 2.0;
            let minor_radius = 1.0;
            NamedSdf::new(SdfTorusXz::new(major_radius, minor_radius), UniqueName::new("torus".to_string()))
        };
        registrator.add(&xz_torus);

        let capped_xy_torus = {
            let major_radius = 2.0;
            let minor_radius = 1.0;
            let cut_angle = Deg(110.0);
            NamedSdf::new(SdfCappedTorusXy::new(cut_angle, major_radius, minor_radius), UniqueName::new("capped_torus".to_string()))
        };
        registrator.add(&capped_xy_torus);
        
        let round_box = NamedSdf::new(SdfRoundBox::new(Vector::new(1.5, 1.0, 1.0), 0.4), UniqueName::new("round_box".to_string()));
        registrator.add(&round_box);

        let link = NamedSdf::new(SdfLink::new(1.0, 0.5, 0.3), UniqueName::new("link".to_string()));
        registrator.add(&link);
        
        let cone = NamedSdf::new(SdfCone::new(Deg(45.0), 1.0), UniqueName::new("cone".to_string()));
        registrator.add(&cone);
        
        let hex_prism = NamedSdf::new(SdfHexPrism::new(1.0, 1.0), UniqueName::new("hex_prism".to_string()));
        registrator.add(&hex_prism);

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
}

impl Materials {
    #[must_use]
    pub(super) fn new(scene: &mut Container) -> Self {
        let gold_metal = scene.materials().add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(1.0, 0.5, 0.0)
                .with_specular_strength(0.00001)
                .with_roughness(-1.0 / 4.0)
                .with_refractive_index_eta(0.0),
        );

        let blue_glass = scene
            .materials()
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(0.0, 0.5, 0.9).with_refractive_index_eta(1.4));

        let purple_glass = scene
            .materials()
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.5, 0.9).with_refractive_index_eta(1.4));

        let red_glass = scene
            .materials()
            .add(&Material::new().with_class(MaterialClass::Glass).with_albedo(1.0, 0.2, 0.0).with_refractive_index_eta(1.4));

        let green_mirror = scene.materials().add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.64, 0.77, 0.22)
                .with_refractive_index_eta(1.4),
        );

        let light_material = scene.materials().add(
            &Material::new()
                .with_emission(2.0, 2.0, 2.0)
        );

        let black_material = scene.materials().add(
            &Material::new()
                .with_albedo(0.2, 0.2, 0.2)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let coral_material = scene.materials().add(
            &Material::new()
                .with_albedo(1.0, 0.5, 0.3)
                .with_specular(0.2, 0.2, 0.2)
                .with_specular_strength(0.01)
                .with_roughness(0.0),
        );

        let red_material = scene.materials().add(
            &Material::new()
                .with_albedo(0.75, 0.1, 0.1)
                .with_specular(0.75, 0.1, 0.1)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let blue_material = scene.materials().add(
            &Material::new()
                .with_albedo(0.1, 0.1, 0.75)
                .with_specular(0.1, 0.1, 0.75)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let bright_red_material = scene.materials().add(
            &Material::new()
                .with_albedo(1.0, 0.0, 0.0)
                .with_specular(1.0, 1.0, 1.0)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let silver_material = scene.materials().add(
            &Material::new()
                .with_class(MaterialClass::Mirror)
                .with_albedo(0.75, 0.75, 0.75)
                .with_specular(0.75, 0.75, 0.75)
                .with_specular_strength(0.55)
                .with_roughness(0.0),
        );

        let green_material = scene.materials().add(
            &Material::new()
                .with_albedo(0.05, 0.55, 0.05)
                .with_specular(0.05, 0.55, 0.05)
                .with_specular_strength(0.05)
                .with_roughness(0.95),
        );

        let selected_object_material = scene.materials().add(
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
        }
    }
}

pub(super) struct World {
    sdf_classes: SdfClasses,
    materials: Materials,
}

impl World {
    #[must_use]
    pub(super) fn new(sdf_classes: SdfClasses, materials: Materials) -> Self {
        Self { sdf_classes, materials }
    }

    #[must_use]
    pub(super) fn selected_object_material(&self) -> MaterialIndex {
        self.materials.selected_object_material
    }

    pub(super) fn switch_to_sdf_exhibition_scene(&self, scene: &mut Container) {
        scene.clear_objects();

        scene.add_parallelogram(Point::new(-1.0, 1.0, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), self.materials.light_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), self.materials.black_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), self.materials.red_material);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), self.materials.green_material);

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
            &(Affine::from_translation(Vector::new(1.3, 0.75, -1.0))*Affine::from_scale(0.1)),
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
    }
    
    pub(super) fn switch_to_ui_box_scene(&self, scene: &mut Container) {
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
            &(Affine::from_translation(Vector::new(0.5, 0.0, -1.0))*Affine::from_scale(0.25) ),
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

        scene.add_parallelogram(Point::new(-1.0, 1.0, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 0.0, 1.0), self.materials.light_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -1.0), Vector::new(3.0, 0.0, 0.0), Vector::new(0.0, 2.1, 0.0), self.materials.black_material);
        scene.add_parallelogram(Point::new(-1.0, -1.1, -0.5), Vector::new(0.0, 0.0, -0.5), Vector::new(0.0, 2.1, 0.0), self.materials.red_material);
        scene.add_parallelogram(Point::new(2.0, -1.1, -1.0), Vector::new(0.0, 0.0, 0.5), Vector::new(0.0, 2.1, 0.0), self.materials.green_material);

        #[must_use]
        fn get_resource_path(file_name: impl AsRef<Path>) -> PathBuf {
            let exe_path = env::current_exe().unwrap();
            let exe_directory = exe_path.parent().unwrap();
            exe_directory.join(file_name)
        }

        let mut meshes = MeshWarehouse::new();
        let cube_mesh_file = get_resource_path(Path::new("assets").join("cube.obj"));
        let cube_mesh_or_error = meshes.load(cube_mesh_file);

        match cube_mesh_or_error {
            Ok(cube_mesh) => {
                let large_box_material = scene.materials().add(&Material::new()
                    .with_albedo(0.95, 0.95, 0.95)
                    .with_refractive_index_eta(2.5));
                let large_box_location =
                    Transformation::new(
                        Affine::from_translation(Vector::new(0.15, 0.6, -1.0)) *
                            Affine::from_nonuniform_scale(3.65, 0.8, 0.25));
                scene.add_mesh(&meshes, cube_mesh, &large_box_location, large_box_material);

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

use bevy::asset;
use bevy::prelude::{App, Plugin};

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};
use bevy_egui::EguiContexts;
use common::store::tokens;

use super::robot::Robot;

pub struct OrientationPlugin;

impl Plugin for OrientationPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_system(rotator_system);
        app.add_system(draw_window);
    }
}

#[derive(Resource)]
pub struct OrientationDisplay(pub Handle<Image>); // Marks the main pass cube, to which the texture is applied.

#[derive(Component)]
struct Object;

// Modified from render_to_texture example
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The cube that will be rendered to the texture.
    commands.spawn((
        PbrBundle {
            mesh: asset_server.load("NAVIGATOR-STACK.STL"),
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        },
        Object,
        first_pass_layer,
    ));

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            camera: Camera {
                // render before the "main pass" camera
                order: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(150.0, 150.0, 150.0))
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..default()
        },
        first_pass_layer,
    ));

    commands.insert_resource(OrientationDisplay(image_handle));
}

fn rotator_system(robot: Res<Robot>, mut query: Query<&mut Transform, With<Object>>) {
    let orientation = robot.store().get(&tokens::ORIENTATION);

    if let Some(data) = orientation {
        let (orientation, _) = &*data;
        let quat = Quat::from_xyzw(
            orientation.0.i as f32,
            orientation.0.j as f32,
            orientation.0.k as f32,
            orientation.0.w as f32,
        );

        for mut transform in &mut query {
            transform.rotation = quat;
        }
    }
}

fn draw_window(mut egui_context: EguiContexts, display: Res<OrientationDisplay>) {
    let texture = egui_context.add_image(display.0.clone_weak());
    let ctx = egui_context.ctx_mut();

    egui::Window::new("Orientation").show(ctx, |ui| ui.image(texture, (512.0, 512.0)));
}

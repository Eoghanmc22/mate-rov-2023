use bevy::prelude::{App, Plugin};

use bevy::{
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
use egui::TextureId;

use super::robot::Robot;

pub struct OrientationPlugin;

impl Plugin for OrientationPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_system(rotator_system);
    }
}

#[derive(Resource, Debug, Clone)]
pub struct OrientationDisplay(pub Handle<Image>, pub TextureId);

// Marks the main pass cube, to which the texture is applied.
#[derive(Component)]
struct Navigator;

// Modified from render_to_texture example
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut egui_context: EguiContexts,
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

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // object
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("NAVIGATOR-STACK.glb#Scene0"),
            transform: Transform::from_scale(Vec3::splat(10.0)),
            ..default()
        },
        Navigator,
        first_pass_layer,
    ));

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Z),
        camera: Camera {
            // render before the "main pass" camera
            order: -1,
            target: RenderTarget::Image(image_handle.clone()),
            ..default()
        },
        ..default()
    });

    // The cube that will be rendered to the texture.
    commands.spawn((
        SceneBundle {
            scene: asset_server.load("NAVIGATOR-STACK.glb"),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        },
        Navigator,
        first_pass_layer,
    ));

    let texture = egui_context.add_image(image_handle.clone_weak());
    commands.insert_resource(OrientationDisplay(image_handle, texture));
}

fn rotator_system(robot: Res<Robot>, mut query: Query<&mut Transform, With<Navigator>>) {
    let orientation = robot.store().get(&tokens::ORIENTATION);

    if let Some(orientation) = orientation {
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

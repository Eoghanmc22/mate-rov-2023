use bevy::prelude::{App, Plugin};

use bevy::scene::SceneInstance;
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
        app.add_system(add_scene_to_render_layer);
        app.add_system(rotator_system);
    }
}

#[derive(Resource, Debug, Clone)]
pub struct OrientationDisplay(pub Handle<Image>, pub TextureId);

#[derive(Component)]
struct NavigatorMarker;

#[derive(Component)]
struct RenderLayerAdded;

// Modified from render_to_texture example
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
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
            scene: asset_server.load("robot.glb#Scene0"),
            transform: Transform::from_scale(Vec3::splat(10.0)),
            ..default()
        },
        NavigatorMarker,
        first_pass_layer,
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Box::new(1.0, 0.1, 0.1).into()),
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            transform: Transform::from_xyz(0.5, 0.0, 0.0),
            ..default()
        },
        first_pass_layer,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Box::new(0.1, 1.0, 0.1).into()),
            material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        first_pass_layer,
    ));
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::Box::new(0.1, 0.1, 1.0).into()),
            material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.5),
            ..default()
        },
        first_pass_layer,
    ));

    // light
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 4000.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(4.0, 4.0, 8.0),
            ..default()
        },
        first_pass_layer,
    ));

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(5.0, -5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Z),
            camera: Camera {
                // render before the "main pass" camera
                order: -1,
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            ..default()
        },
        first_pass_layer,
    ));

    let texture = egui_context.add_image(image_handle.clone_weak());
    commands.insert_resource(OrientationDisplay(image_handle, texture));
}

fn add_scene_to_render_layer(
    mut commands: Commands,
    scenes: Res<SceneSpawner>,
    query: Query<(Entity, &SceneInstance), (With<SceneInstance>, Without<RenderLayerAdded>)>,
) {
    for (entity, instance) in query.iter() {
        if scenes.instance_is_ready(**instance) {
            let first_pass_layer = RenderLayers::layer(1);

            for entity in scenes.iter_instance_entities(**instance) {
                commands.entity(entity).insert(first_pass_layer);
            }

            commands.entity(entity).insert(RenderLayerAdded);
        }
    }
}

fn rotator_system(robot: Res<Robot>, mut query: Query<&mut Transform, With<NavigatorMarker>>) {
    let orientation = robot.store().get(&tokens::ORIENTATION);

    if let Some(orientation) = orientation {
        for mut transform in &mut query {
            transform.rotation = orientation.0.into();
        }
    }
}

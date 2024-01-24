use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::Indices, render_resource::PrimitiveTopology},
    sprite::Mesh2dHandle,
};
use bevy_pancam::{PanCam, PanCamPlugin};
use itertools::Itertools;

use crate::{
    simulation::{
        level::{Enemy, Level, Wall, ENEMY_RADIUS},
        SimulationStartupSet,
    },
    utils::Vertices,
};

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PanCamPlugin,))
            .insert_resource(GizmoConfig {
                line_width: 2.,
                ..default()
            })
            .insert_resource(ClearColor(Color::rgb_u8(222, 220, 227)))
            .add_systems(
                Startup,
                (
                    spawn_camera,
                    (add_wall_meshes,).after(SimulationStartupSet::Flush),
                ),
            )
            .add_systems(Update, (print_cam_pos, add_enemy_sprites));
    }
}

fn spawn_camera(mut commands: Commands, level: Res<Level>) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(level.size.x / 2., level.size.y / 2., 100.),
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(level.size.y),
                ..default()
            },
            ..default()
        },
        PanCam::default(),
    ));
}

fn print_cam_pos(mut _query: Query<&Transform, With<Camera>>) {
    // for cam in query.iter_mut() {
    //     println!("cam pos: {:?}", cam.translation);
    // }
}

fn add_enemy_sprites(
    mut commands: Commands,
    new_enemy_q: Query<Entity, Added<Enemy>>,
    asset_server: Res<AssetServer>,
) {
    for entity in new_enemy_q.iter() {
        commands.entity(entity).insert((
            Sprite {
                color: Color::rgb_u8(224, 49, 29).with_a(0.4),
                custom_size: Some(Vec2::splat(ENEMY_RADIUS * 2.)),
                ..default()
            },
            asset_server.load::<Image>("circle.png"),
        ));
    }
}

fn make_triangulated_mesh(vertices: &Vertices) -> anyhow::Result<Mesh> {
    let center = vertices.iter().sum::<Vec2>() / vertices.len() as f32;
    let flat_vertices = vertices
        .iter()
        .flat_map(|v| [v.x - center.x, v.y - center.y])
        .collect::<Vec<f32>>();

    let Ok(triangulated_indices) = earcutr::earcut(&flat_vertices, &[], 2) else {
        anyhow::bail!("Failed to triangulate vertices");
    };

    let triangulated_indices = triangulated_indices
        .iter()
        .map(|i| *i as u32)
        .collect::<Vec<_>>();

    let mesh_vert = flat_vertices
        .iter()
        .chunks(2)
        .into_iter()
        .map(|mut it| [*it.next().unwrap(), *it.next().unwrap(), 0.0])
        .collect::<Vec<[f32; 3]>>();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_vert);
    mesh.set_indices(Some(Indices::U32(triangulated_indices)));
    Ok(mesh)
}

fn add_wall_meshes(
    new_wall_q: Query<(Entity, &Wall), Added<Wall>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    let material = materials.add(ColorMaterial::from(Color::BLACK.with_a(0.8)));
    for (entity, wall) in new_wall_q.iter() {
        commands.entity(entity).insert((
            Mesh2dHandle(meshes.add(make_triangulated_mesh(&wall.0).unwrap())),
            material.clone(),
        ));
    }
}

use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::Indices, render_resource::PrimitiveTopology},
    sprite::Mesh2dHandle,
};
use bevy_pancam::{PanCam, PanCamPlugin};
use itertools::Itertools;

use crate::{
    simulation::{
        level::{Enemy, Level, SpawnPoint, Target, Wall, ENEMY_RADIUS},
        navigation::{Flow, FlowField, NavGrid, NavGridInner, NAV_SCALE},
        SimulationStartupSet,
    },
    utils::{square, Vertices, WithOffset},
};

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PanCamPlugin,))
            .insert_resource(GizmoConfig {
                line_width: 2.0,
                ..default()
            })
            .insert_resource(ClearColor(Color::rgb_u8(222, 220, 227)))
            .insert_resource(ShowFlowField(true))
            .add_systems(
                Startup,
                (
                    spawn_camera,
                    (add_wall_meshes, add_target_sprites, add_spawn_point_sprites)
                        .after(SimulationStartupSet::Flush),
                ),
            )
            .add_systems(
                Update,
                (
                    print_cam_pos,
                    add_enemy_sprites,
                    draw_level_bounds,
                    toggle_show_flow_field,
                    draw_flow_field.run_if(resource_equals(ShowFlowField(true))),
                ),
            );
    }
}

#[derive(Resource, PartialEq, Eq)]
struct ShowFlowField(bool);

fn spawn_camera(mut commands: Commands, level: Res<Level>) {
    commands.spawn((
        Camera2dBundle {
            transform: Transform::from_xyz(level.size / 2., level.size / 2., 100.),
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(level.size),
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

fn add_target_sprites(
    mut commands: Commands,
    new_target_q: Query<Entity, Added<Target>>,
    asset_server: Res<AssetServer>,
) {
    for entity in new_target_q.iter() {
        commands.entity(entity).insert((
            Sprite {
                color: Color::rgb_u8(31, 39, 240).with_a(0.8),
                custom_size: Some(Vec2::splat(ENEMY_RADIUS * 2.)),
                ..default()
            },
            asset_server.load::<Image>("cross.png"),
        ));
    }
}

fn add_spawn_point_sprites(
    mut commands: Commands,
    new_spawn_point_q: Query<Entity, Added<SpawnPoint>>,
    asset_server: Res<AssetServer>,
) {
    for entity in new_spawn_point_q.iter() {
        commands.entity(entity).insert((
            Sprite {
                color: Color::rgb_u8(136, 2, 214).with_a(0.8),
                custom_size: Some(Vec2::splat(ENEMY_RADIUS * 2.)),
                ..default()
            },
            asset_server.load::<Image>("hollow_circle.png"),
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

fn draw_level_bounds(mut gizmos: Gizmos, level: Res<Level>) {
    let bounds = square(level.size).with_offset(Vec2::splat(level.size / 2.));
    for (p1, p2) in bounds.iter().zip(bounds.iter().cycle().skip(1)) {
        gizmos.line_2d(*p1, *p2, Color::BLACK);
    }
}

pub fn draw_gizmo_cross(gizmos: &mut Gizmos, pos: Vec2, color: Color, size: f32) {
    gizmos.line_2d(
        pos - Vec2::new(0.0, size),
        pos + Vec2::new(0.0, size),
        color,
    );
    gizmos.line_2d(
        pos - Vec2::new(size, 0.0),
        pos + Vec2::new(size, 0.0),
        color,
    );
}

fn toggle_show_flow_field(input: Res<Input<KeyCode>>, mut show_flow_field: ResMut<ShowFlowField>) {
    if input.just_pressed(KeyCode::F) {
        println!("Toggling flow field");
        show_flow_field.0 = !show_flow_field.0;
    }
}

fn draw_flow_field(
    flow_field: Res<FlowField>,
    nav_grid: Res<NavGrid>,
    mut gizmos: Gizmos,
    camera_q: Query<&Transform, With<Camera>>,
) {
    let camera_tr = camera_q.single();
    let camera_pos_index = nav_grid.pos_to_index(camera_tr.translation.truncate());

    let cx = camera_pos_index[0];
    let cy = camera_pos_index[1];

    let extent = 140;

    for x in cx.saturating_sub(extent)..(cx + extent).min(flow_field.0.shape()[0]) {
        for y in cy.saturating_sub(extent)..(cy + extent).min(flow_field.0.shape()[1]) {
            let (_dist, flow) = flow_field.0[[x, y]];
            let pos = NavGridInner::index_to_pos([x, y]);
            if flow == Flow::None {
                gizmos.line_2d(
                    pos - Vec2::new(0.0, 0.2),
                    pos + Vec2::new(0.0, 0.2),
                    Color::RED,
                );
                continue;
            }

            if flow == Flow::Source {
                draw_gizmo_cross(&mut gizmos, pos, Color::GREEN, NAV_SCALE);
                continue;
            }

            let dir = flow.to_dir() * NAV_SCALE * 2.0f32.sqrt() * 0.5;
            gizmos.line_gradient_2d(pos - dir, pos + dir, Color::BLACK.with_a(0.5), Color::BLACK);
        }
    }
}

use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

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
    statistics::Statistics,
    utils::{square, Easing, ToAngle, Vertices, WithOffset},
    Ticks,
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
                    (spawn_camera, init_diagnostics_text),
                    (add_wall_meshes, add_target_sprites, add_spawn_point_sprites)
                        .after(SimulationStartupSet::Flush),
                ),
            )
            .add_systems(
                Update,
                (
                    add_enemy_sprites,
                    draw_level_bounds,
                    toggle_show_flow_field,
                    draw_flow_field.run_if(resource_equals(ShowFlowField(true))),
                    update_diagnostics_text,
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
    let size = size * 0.5;
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
        info!("Toggling flow field");
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

    let range_x = cx.saturating_sub(extent)..(cx + extent).min(flow_field.0.shape()[0]);
    let range_y = cy.saturating_sub(extent)..(cy + extent).min(flow_field.0.shape()[1]);

    // Draw grid lines
    for x in range_x.clone() {
        let pos = NavGridInner::index_to_pos([x, range_y.start]) + Vec2::splat(NAV_SCALE * 0.5);
        let end_pos =
            NavGridInner::index_to_pos([x, range_y.end - 1]) + Vec2::splat(NAV_SCALE * 0.5);
        gizmos.line_2d(pos, end_pos, Color::BLACK.with_a(0.2));
    }
    for y in range_y.clone() {
        let pos = NavGridInner::index_to_pos([range_x.start, y]) + Vec2::splat(NAV_SCALE * 0.5);
        let end_pos =
            NavGridInner::index_to_pos([range_x.end - 1, y]) + Vec2::splat(NAV_SCALE * 0.5);
        gizmos.line_2d(pos, end_pos, Color::BLACK.with_a(0.2));
    }

    // Draw flow field
    for x in range_x.clone() {
        for y in range_y.clone() {
            let (dist, flow) = flow_field.0[[x, y]];
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

            let angle = flow.to_dir().to_angle().rem_euclid(FRAC_PI_2);
            let dir = flow.to_dir() * (1. / angle.cos().max(angle.sin())) * 0.5 * NAV_SCALE;
            gizmos.line_2d(
                pos - dir,
                pos + dir,
                Color::SEA_GREEN.lerp(Color::BLACK, dist / 100.),
            );
        }
    }
}

#[derive(Component)]
struct StatsText;

fn init_diagnostics_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                left: Val::Px(5.),
                top: Val::Px(5.),
                width: Val::Px(300.),
                padding: UiRect::all(Val::Px(5.)),
                position_type: PositionType::Absolute,
                ..default()
            },
            background_color: Color::BLACK.with_a(0.9).into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                StatsText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: asset_server.load("Hack-Regular.ttf"),
                        font_size: 18.0,
                        color: Color::WHITE,
                    },
                ),
            ));
        });
}

fn avg20(v: &[Duration]) -> Duration {
    let sum = v.iter().rev().take(20).sum::<Duration>();
    sum / v.len() as u32
}

fn update_diagnostics_text(
    mut text_q: Query<&mut Text, With<StatsText>>,
    stats: Res<Statistics>,
    tick: Res<Ticks>,
) {
    let mut text = text_q.single_mut();

    text.sections[0].value = format!("tick: {}\n", tick.0)
        + &stats
            .0
            .iter()
            .map(|(k, v)| format!("{k:14 }: {:?}", avg20(v)))
            .sorted()
            .join("\n");
}

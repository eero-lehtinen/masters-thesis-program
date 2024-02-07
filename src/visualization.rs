use std::f32::consts::PI;
use std::time::Duration;

use bevy::{
    core::FrameCount,
    prelude::*,
    render::{
        camera::ScalingMode,
        mesh::Indices,
        render_resource::{Extent3d, PrimitiveTopology, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
    sprite::Mesh2dHandle,
};
use bevy_pancam::{PanCam, PanCamPlugin};
use itertools::Itertools;

use crate::{
    simulation::navigation::{Flow, FlowField, NavGrid, NavGridInner, NAV_SCALE, NAV_SCALE_INV},
    statistics::Statistics,
    utils::{square, Vertices, WithOffset},
    Command,
};

use crate::simulation::spawning::{Enemy, ENEMY_RADIUS};

use crate::level::*;

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PanCamPlugin,))
            .insert_resource(GizmoConfig {
                line_width: 2.0,
                ..default()
            })
            .insert_resource(ClearColor(Color::hex("#303030").unwrap()))
            .insert_resource(ShowFlowFieldLines(false))
            .add_systems(
                Startup,
                (
                    (spawn_camera, init_diagnostics_text),
                    (add_wall_meshes, add_flow_field_sprite),
                ),
            )
            .add_systems(
                Update,
                (
                    add_target_sprites,
                    add_spawn_point_sprites,
                    add_enemy_sprites,
                    draw_level_bounds,
                    toggle_show_flow_field,
                    dran_nav_grid,
                    update_flow_field_color,
                    draw_flow_field_gizmos.run_if(resource_equals(ShowFlowFieldLines(true))),
                    update_diagnostics_text,
                ),
            );
    }
}

#[derive(Resource, PartialEq, Eq)]
struct ShowFlowFieldLines(bool);

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
        PanCam {
            grab_buttons: vec![MouseButton::Right, MouseButton::Middle],
            ..default()
        },
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
                color: Color::rgb_u8(55, 41, 255).with_a(0.6),
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
                color: Color::hex("#3A90E0").unwrap().with_a(0.8),
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
                color: Color::hex("#8E2CD8").unwrap().with_a(0.8),
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
    command: Res<Command>,
) {
    if *command == Command::Editor {
        return;
    }
    let material = materials.add(ColorMaterial::from(Color::GRAY.with_a(0.4)));
    for (entity, wall) in new_wall_q.iter() {
        commands.entity(entity).insert((
            Mesh2dHandle(meshes.add(make_triangulated_mesh(&wall.0).unwrap())),
            material.clone(),
        ));
    }
}

fn draw_level_bounds(mut gizmos: Gizmos, level_size: Res<LevelSize>) {
    let bounds = square(level_size.0).with_offset(Vec2::splat(level_size.0 / 2.));
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

fn toggle_show_flow_field(
    input: Res<Input<KeyCode>>,
    mut show_flow_field: ResMut<ShowFlowFieldLines>,
    flow_field: Option<Res<FlowField>>,
) {
    if flow_field.is_none() {
        return;
    }
    if input.just_pressed(KeyCode::F) {
        info!("Toggling flow field");
        show_flow_field.0 = !show_flow_field.0;
    }
}

#[derive(Component)]
struct FlowFieldSprite;

fn add_flow_field_sprite(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    nav_grid: Option<Res<NavGrid>>,
    level: Res<Level>,
) {
    let Some(nav_grid) = nav_grid else {
        return;
    };
    let mut image = Image::new_fill(
        Extent3d {
            width: nav_grid.0.grid.dim().0 as u32,
            height: nav_grid.0.grid.dim().1 as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);

    let size = level.size + 2. * NAV_SCALE;

    commands.spawn((
        FlowFieldSprite,
        SpriteBundle {
            texture: handle,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            transform: Transform::from_xyz(size / 2. - NAV_SCALE, size / 2. - NAV_SCALE, 0.),
            ..default()
        },
    ));
}

fn dran_nav_grid(level_size: Res<LevelSize>, mut gizmos: Gizmos) {
    let width = (level_size.0 * NAV_SCALE_INV) as i32 + 2;
    let height = width;
    // Draw grid lines
    for x in 0..width {
        let pos = Vec2::new((x as f32 - 1.) * NAV_SCALE, -NAV_SCALE);
        let end_pos = Vec2::new(
            (x as f32 - 1.) * NAV_SCALE,
            (height as f32 - 1.) * NAV_SCALE,
        );
        gizmos.line_2d(pos, end_pos, Color::BLACK.with_a(0.2));
    }
    for y in 0..height {
        let pos = Vec2::new(-NAV_SCALE, (y as f32 - 1.) * NAV_SCALE);
        let end_pos = Vec2::new((width as f32 - 1.) * NAV_SCALE, (y as f32 - 1.) * NAV_SCALE);
        gizmos.line_2d(pos, end_pos, Color::BLACK.with_a(0.2));
    }
}

fn update_flow_field_color(
    flow_field: Option<Res<FlowField>>,
    sprite_q: Query<&Handle<Image>, With<FlowFieldSprite>>,
    mut images: ResMut<Assets<Image>>,
    level: Res<Level>,
) {
    let Some(flow_field) = flow_field else {
        return;
    };

    let (width, _) = flow_field.0.dim();

    let image_handle = sprite_q.single();
    let image = images.get_mut(image_handle).unwrap();
    let mut change_pixel = move |x, y, color: [u8; 4]| {
        let y = image.height() as usize - y - 1;
        let pixel = &mut image.data[(x + y * width) * 4..(x + y * width) * 4 + 4];
        pixel.copy_from_slice(&color);
    };

    let max_dist = level.size * NAV_SCALE_INV * 2.0f32.sqrt();

    flow_field
        .0
        .indexed_iter()
        .for_each(|((x, y), &(dist, flow))| {
            if flow == Flow::None {
                change_pixel(x, y, [0, 0, 0, 255]);
                return;
            }

            if flow == Flow::Source {
                change_pixel(x, y, [0, 255, 0, 255]);
                return;
            }

            let dist = dist.min(max_dist) / max_dist;
            let color = Color::hsla((1. - dist) * 120., 1., 0.5, 1.);
            change_pixel(x, y, color.as_rgba_u8());
        });
}

fn draw_flow_field_gizmos(
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

    let (width, height) = flow_field.0.dim();

    let range_x = cx.saturating_sub(extent)..(cx + extent).min(width);
    let range_y = cy.saturating_sub(extent)..(cy + extent).min(height);

    // Draw flow field
    for x in range_x.clone() {
        for y in range_y.clone() {
            let (_, flow) = flow_field.0[[x, y]];
            let pos = NavGridInner::index_to_pos([x, y]);
            if flow == Flow::None {
                gizmos.line_2d(
                    pos - Vec2::new(0.0, 0.1),
                    pos + Vec2::new(0.0, 0.1),
                    Color::RED,
                );
                continue;
            }

            if flow == Flow::Source {
                draw_gizmo_cross(&mut gizmos, pos, Color::BLACK, NAV_SCALE);
                continue;
            }

            let dir = flow.to_dir() * 0.45 * NAV_SCALE;
            gizmo_arrow(&mut gizmos, pos - dir, pos + dir, Color::BLACK);
        }
    }
}

fn gizmo_arrow(gizmos: &mut Gizmos, from: Vec2, to: Vec2, color: Color) {
    gizmos.line_2d(from, to, color);
    let dir = (to - from).normalize();
    let arrow_head = dir * 0.16;
    let arrow = Vec2::from_angle(-PI / 5.).rotate(arrow_head);
    gizmos.line_2d(to, to - arrow, color);
    let arrow = Vec2::from_angle(PI / 5.).rotate(arrow_head);
    gizmos.line_2d(to, to - arrow, color);
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

fn avg_last_n(v: &[Duration], n: u32) -> Duration {
    let sum = v.iter().rev().take(n as usize).sum::<Duration>();
    sum / n
}

fn update_diagnostics_text(
    mut text_q: Query<&mut Text, With<StatsText>>,
    stats: Option<Res<Statistics>>,
    tick: Res<FrameCount>,
) {
    let Some(stats) = stats else {
        return;
    };
    let mut text = text_q.single_mut();

    let avgs = stats
        .0
        .iter()
        .sorted()
        .map(|(k, v)| (k, avg_last_n(v, 20)))
        .collect_vec();

    let total = avgs.iter().map(|(_, v)| v).sum::<Duration>();

    let mut value = format!("tick: {}\n", tick.0);

    value += &avgs
        .iter()
        .map(|(k, v)| format!("{k:14 }: {:?}", v))
        .join("\n");

    value += &format!(
        "\ntotal: {:.2} ms, fps: {:.1}",
        total.as_secs_f64() * 1000.,
        1. / total.as_secs_f64()
    );

    text.sections[0].value = value;
}

use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;
use rand::Rng;

use crate::utils::Vertices;

use super::{rng::FastRng, SimulationSet};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Level::get())
            .add_systems(Startup, spawn_level)
            .add_systems(PreUpdate, spawn_enemies.in_set(SimulationSet::Spawn));
    }
}

#[derive(Resource)]
pub struct Level {
    pub size: Vec2,
    pub spawn_point: Vec2,
    pub target: Vec2,
    pub walls: Vec<Vertices>,
}

impl Level {
    fn get() -> Level {
        Level {
            size: Vec2::new(100., 100.),
            spawn_point: Vec2::new(10., 10.),
            target: Vec2::new(90., 90.),
            walls: vec![square(10.).with_offset(Vec2::new(50., 50.))],
        }
    }
}

#[derive(Component, Debug)]
struct SpawnPoint;

#[derive(Component, Debug)]
struct Target;

#[derive(Component, Debug)]
pub struct Enemy;

fn square(size: f32) -> Vertices {
    rectangle(Vec2::splat(size))
}

fn rectangle(size: Vec2) -> Vertices {
    let half_width = size.x / 2.;
    let half_height = size.y / 2.;
    vec![
        Vec2::new(-half_width, -half_height),
        Vec2::new(half_width, -half_height),
        Vec2::new(half_width, half_height),
        Vec2::new(-half_width, half_height),
    ]
}

trait WithOffset {
    fn with_offset(self, offset: Vec2) -> Self;
}

impl WithOffset for Vertices {
    fn with_offset(self, offset: Vec2) -> Self {
        self.into_iter().map(|v| v + offset).collect()
    }
}

fn polyline_collider(vertices: Vertices) -> Collider {
    let mut collider_indices = Vec::new();
    for i in 0..vertices.len() - 1 {
        collider_indices.push([i as u32, i as u32 + 1]);
    }
    collider_indices.push([vertices.len() as u32 - 1, 0]);
    Collider::polyline(vertices, Some(collider_indices))
}

fn spatial(pos: Vec2) -> SpatialBundle {
    SpatialBundle {
        transform: Transform::from_translation(pos.extend(0.)),
        ..default()
    }
}

fn spawn_level(level: Res<Level>, mut commands: Commands) {
    commands.spawn((SpawnPoint, spatial(Vec2::new(10., 10.))));
    commands.spawn((Target, spatial(Vec2::new(100., 100.))));

    for wall in &level.walls {
        let center = wall.iter().sum::<Vec2>() / wall.len() as f32;
        commands.spawn((
            polyline_collider(wall.clone().with_offset(-center)),
            spatial(center),
        ));
    }

    commands.spawn((
        polyline_collider(rectangle(level.size)),
        spatial(level.size / 2.),
    ));
}

pub const ENEMY_RADIUS: f32 = 1.;

pub const MAX_ENEMIES: u32 = 10_000;

fn spawn_enemies(
    spawn_point_q: Query<&Transform, With<SpawnPoint>>,
    mut commands: Commands,
    mut rng: Local<FastRng>,
    mut count: Local<u32>,
    asset_server: Res<AssetServer>,
) {
    if *count >= MAX_ENEMIES {
        return;
    }
    *count += 1;

    let spawn_point = spawn_point_q.single();
    let offset = Vec2::new(rng.gen_range(-0.5..0.5), rng.gen_range(-0.5..0.5));

    commands.spawn((
        Enemy,
        Collider::ball(ENEMY_RADIUS),
        spatial(spawn_point.translation.truncate() + offset),
    ));
}

use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;
use rand::{seq::IteratorRandom, Rng};

use crate::utils::{rectangle, square, Velocity, Vertices, WithOffset};

use super::{navigation::NavGrid, rng::FastRng, SimulationSet, SimulationStartupSet};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Level::get())
            .add_systems(Startup, spawn_level.in_set(SimulationStartupSet::Spawn))
            .add_systems(
                PreUpdate,
                (
                    spawn_enemies.in_set(SimulationSet::Spawn),
                    despawn_on_target_enemies.in_set(SimulationSet::Despawn),
                ),
            );
    }
}

#[derive(Resource)]
pub struct Level {
    pub size: f32,
    pub spawn_points: Vec<Vec2>,
    pub targets: Vec<Vec2>,
    pub walls: Vec<Vertices>,
}

impl Level {
    fn get() -> Level {
        Level {
            size: 100.,
            spawn_points: vec![
                Vec2::new(10., 10.),
                Vec2::new(10., 20.),
                Vec2::new(20., 10.),
            ],
            targets: vec![
                Vec2::new(90., 90.),
                Vec2::new(80., 90.),
                Vec2::new(90., 80.),
            ],
            walls: vec![square(50.).with_offset(Vec2::new(50., 50.))],
        }
        .scaled(3.)
    }

    fn scaled(self, scale: f32) -> Level {
        Level {
            size: self.size * scale,
            spawn_points: self.spawn_points.into_iter().map(|p| p * scale).collect(),
            targets: self.targets.into_iter().map(|p| p * scale).collect(),
            walls: self
                .walls
                .into_iter()
                .map(|v| v.into_iter().map(|p| p * scale).collect())
                .collect(),
        }
    }
}

#[derive(Component, Debug)]
pub struct SpawnPoint;

#[derive(Component, Debug)]
pub struct Target;

#[derive(Component, Debug)]
pub struct Enemy;

#[derive(Component, Debug)]
pub struct Wall(pub Vertices);

fn polyline_collider(vertices: Vertices) -> Collider {
    let mut collider_indices = Vec::new();
    for i in 0..vertices.len() - 1 {
        collider_indices.push([i as u32, i as u32 + 1]);
    }
    collider_indices.push([vertices.len() as u32 - 1, 0]);
    Collider::polyline(vertices, Some(collider_indices))
}

fn spatial(pos: Vec2, z: f32) -> SpatialBundle {
    SpatialBundle {
        transform: Transform::from_translation(pos.extend(z)),
        ..default()
    }
}

fn spawn_level(level: Res<Level>, mut commands: Commands) {
    for spawn_point in &level.spawn_points {
        commands.spawn((SpawnPoint, spatial(*spawn_point, 2.)));
    }
    for target in &level.targets {
        commands.spawn((Target, spatial(*target, 3.)));
    }

    for wall in &level.walls {
        let center = wall.iter().sum::<Vec2>() / wall.len() as f32;
        let local_vertices = wall.clone().with_offset(-center);
        commands.spawn((
            Wall(local_vertices.clone()),
            polyline_collider(local_vertices),
            spatial(center, 1.),
        ));
    }
    commands.spawn((
        polyline_collider(rectangle(Vec2::splat(level.size))),
        spatial(Vec2::splat(level.size / 2.), 1.),
    ));
}

pub const ENEMY_RADIUS: f32 = 0.5;

pub const MAX_ENEMIES: u32 = 10_000;

const SPAWN_PER_TICK: u32 = 50;

fn spawn_enemies(
    spawn_point_q: Query<&Transform, With<SpawnPoint>>,
    mut commands: Commands,
    mut rng: Local<FastRng>,
    mut count: Local<u32>,
) {
    if *count >= MAX_ENEMIES {
        return;
    }
    *count += SPAWN_PER_TICK;

    for _ in 0..SPAWN_PER_TICK {
        let spawn_point = spawn_point_q.iter().choose(&mut rng.0).unwrap();
        let offset = Vec2::new(rng.gen_range(-0.5..0.5), rng.gen_range(-0.5..0.5));

        commands.spawn((
            Enemy,
            Collider::ball(ENEMY_RADIUS),
            spatial(
                spawn_point.translation.truncate() + offset,
                rng.gen_range(1. ..2.),
            ),
            Velocity::default(),
        ));
    }
}

fn despawn_on_target_enemies(
    target_q: Query<&Transform, With<Target>>,
    enemy_q: Query<(Entity, &Transform), With<Enemy>>,
    nav_grid: Res<NavGrid>,
    mut commands: Commands,
) {
    let target_indices = target_q
        .iter()
        .map(|t| nav_grid.pos_to_index(t.translation.truncate()))
        .collect::<Vec<_>>();

    for (entity, transform) in enemy_q.iter() {
        let nav_idx = nav_grid.pos_to_index(transform.translation.truncate());
        if target_indices.contains(&nav_idx) {
            commands.entity(entity).despawn();
        }
    }
}

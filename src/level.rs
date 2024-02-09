use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;
use serde::{Deserialize, Serialize};

use crate::utils::{rectangle, spatial, square, Vertices, WithOffset};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, spawn_level.in_set(LevelStartupSet::Spawn));
    }
}

#[derive(Debug, Default, Resource)]
pub struct LevelSize(pub f32);

#[derive(Debug, Default, Resource)]
pub struct LevelPath(pub String);

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum LevelStartupSet {
    Spawn,
}

#[derive(Resource, Serialize, Deserialize, Clone)]
pub struct Level {
    pub size: f32,
    pub spawn_points: Vec<Vec2>,
    pub targets: Vec<Vec2>,
    pub walls: Vec<Vertices>,
}

impl Default for Level {
    fn default() -> Self {
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
    }
}

impl Level {
    pub fn scale_to(&mut self, size: f32) {
        let scale = size / self.size;
        self.size = size;
        self.spawn_points.iter_mut().for_each(|p| *p *= scale);
        self.targets.iter_mut().for_each(|p| *p *= scale);
        self.walls
            .iter_mut()
            .for_each(|v| v.iter_mut().for_each(|p| *p *= scale));
    }
}

#[derive(Component, Debug)]
pub struct SpawnPoint;

#[derive(Component, Debug)]
pub struct Target;

#[derive(Component, Debug)]
pub struct Wall(pub Vertices);

#[derive(Bundle)]
pub struct SpawnPointBundle {
    pub spawn_point: SpawnPoint,
    pub spatial: SpatialBundle,
}

impl SpawnPointBundle {
    pub fn new(pos: Vec2) -> Self {
        Self {
            spawn_point: SpawnPoint,
            spatial: spatial(pos, 2.),
        }
    }
}

#[derive(Bundle)]
pub struct TargetBundle {
    pub target: Target,
    pub spatial: SpatialBundle,
}

impl TargetBundle {
    pub fn new(pos: Vec2) -> Self {
        Self {
            target: Target,
            spatial: spatial(pos, 3.),
        }
    }
}

#[derive(Bundle)]
pub struct WallBundle {
    pub wall: Wall,
    pub collider: Collider,
    pub spatial: SpatialBundle,
}

impl WallBundle {
    pub fn new(vertices: &Vertices) -> Self {
        let center = vertices.iter().sum::<Vec2>() / vertices.len() as f32;
        let local_vertices = vertices.clone().with_offset(-center);
        Self {
            wall: Wall(local_vertices.clone()),
            collider: polyline_collider(local_vertices),
            spatial: spatial(center, 1.),
        }
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

fn spawn_level(world: &mut World) {
    let level = (*world.get_resource::<Level>().unwrap()).clone();

    for spawn_point in &level.spawn_points {
        world.spawn(SpawnPointBundle::new(*spawn_point));
    }
    for target in &level.targets {
        world.spawn(TargetBundle::new(*target));
    }

    for wall in &level.walls {
        world.spawn(WallBundle::new(wall));
    }

    world.spawn((
        polyline_collider(rectangle(Vec2::splat(level.size))),
        spatial(Vec2::splat(level.size / 2.), 1.),
    ));

    world.insert_resource(LevelSize(level.size));
}

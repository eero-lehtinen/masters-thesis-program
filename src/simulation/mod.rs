use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;

use crate::utils::Vertices;

use self::{collision::CollisionPlugin, navigation::NavigationPlugin};

mod collision;
mod navigation;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((NavigationPlugin, CollisionPlugin))
            .add_systems(Startup, create_level)
            .edit_schedule(PreUpdate, |s| {
                s.configure_sets(
                    (
                        SimulationSet::GenNavigation,
                        SimulationSet::Move,
                        SimulationSet::ApplyColliders,
                    )
                        .chain(),
                );
            });
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum SimulationSet {
    GenNavigation,
    Move,
    ApplyColliders,
}

#[derive(Component, Debug)]
struct SpawnPoint;

#[derive(Component, Debug)]
struct Target;

#[derive(Component, Debug)]
struct Wall(Vertices);

fn square(size: f32) -> Vertices {
    let half = size / 2.;
    vec![
        Vec2::new(-half, -half),
        Vec2::new(half, -half),
        Vec2::new(half, half),
        Vec2::new(-half, half),
    ]
}

fn create_level(mut commands: Commands) {
    let spatial = |x, y| SpatialBundle {
        transform: Transform::from_xyz(x, y, 0.),
        ..default()
    };

    commands.spawn((SpawnPoint, spatial(10., 10.)));
    commands.spawn((Target, spatial(100., 100.)));

    commands.spawn_batch(vec![(
        Collider::polyline(square(10.), None),
        spatial(50., 50.),
    )]);
}

fn spawn_enemies(mut commands: Commands) {}

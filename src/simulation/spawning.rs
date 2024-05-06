use bevy::prelude::*;
use bevy_rapier2d::geometry::Collider;
use rand::{seq::IteratorRandom, Rng, RngCore};

use crate::{
    level::{SpawnPoint, Target},
    utils::{spatial, Velocity},
};

use super::{navigation::NavGrid, rng::FastRng, SimulationSet};

pub struct SpawningPlugin;

impl Plugin for SpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                spawn_enemies.in_set(SimulationSet::Spawn),
                despawn_on_target_enemies.in_set(SimulationSet::Despawn),
            ),
        );
    }
}

#[derive(Component, Debug)]
pub struct Enemy;

#[derive(Bundle)]
pub struct EnemyBundle {
    pub enemy: Enemy,
    pub collider: Collider,
    pub spatial: SpatialBundle,
    pub velocity: Velocity,
}

impl EnemyBundle {
    pub fn new(pos: Vec2, rng: &mut impl RngCore) -> Self {
        let offset = Vec2::new(rng.gen_range(-0.5..0.5), rng.gen_range(-0.5..0.5));
        EnemyBundle {
            enemy: Enemy,
            collider: Collider::ball(ENEMY_RADIUS),
            spatial: spatial(pos + offset, rng.gen_range(1. ..2.)),
            velocity: Velocity::default(),
        }
    }
}

pub const ENEMY_RADIUS: f32 = 0.5;

pub const MAX_ENEMIES: u32 = 10_000;

const SPAWN_PER_TICK: u32 = 300;

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
        let Some(spawn_point) = spawn_point_q.iter().choose(&mut rng.0) else {
            return;
        };

        commands.spawn(EnemyBundle::new(
            spawn_point.translation.truncate(),
            &mut rng.0,
        ));
    }
}

fn despawn_on_target_enemies(
    target_q: Query<&Transform, With<Target>>,
    enemy_q: Query<(Entity, &Transform), With<Enemy>>,
    nav_grid: Option<Res<NavGrid>>,
    nav_grid2: Option<Res<NavGrid>>,
    mut commands: Commands,
) {
    let pos_to_index = |pos: Vec2| {
        if let Some(nav_grid) = &nav_grid {
            nav_grid.pos_to_index(pos)
        } else {
            nav_grid2.as_ref().unwrap().pos_to_index(pos)
        }
    };
    let target_indices = target_q
        .iter()
        .map(|t| pos_to_index(t.translation.truncate()))
        .collect::<Vec<_>>();

    for (entity, transform) in enemy_q.iter() {
        let nav_idx = pos_to_index(transform.translation.truncate());
        if target_indices.contains(&nav_idx) {
            commands.entity(entity).despawn();
        }
    }
}

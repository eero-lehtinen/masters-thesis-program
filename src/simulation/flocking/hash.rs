use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use bevy::utils::HashMap;
// use std::collections::HashMap;

use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use crate::level::*;

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(level: Res<Level>, mut commands: Commands) {
    println!("USING: spatial hash");
    commands.insert_resource(SpatialStructure::new(level.size));
}

pub fn keep_distance_to_others(world: &mut World) {
    let mut system_state: SystemState<(
        Query<(Entity, &mut Transform, &mut Velocity), With<Enemy>>,
        Res<NavGrid>,
        Res<FlowField>,
        ResMut<SpatialStructure>,
        ResMut<Statistics>,
    )> = SystemState::new(world);
    let (mut enemy_q, nav_grid, flow_field, mut spatial, mut stats) = system_state.get_mut(world);

    let start = Instant::now();
    spatial.reset();
    let reset_elapsed = start.elapsed();

    enemy_q
        .iter()
        .for_each(|(entity, tr, _)| spatial.insert((entity, tr.translation.truncate())));
    let insert_elapsed = start.elapsed();

    let pref_dist = PREFERRED_DISTANCE;

    spatial.grid.iter().for_each(|(cell, items)| {
        let neighbors = spatial.neighbors(*cell);

        for &(entity, pos) in items {
            let Ok((_, mut translation, mut velocity)) = enemy_q.get_mut(entity) else {
                continue;
            };

            let neighbors = neighbors.iter().flatten().flat_map(|v| v.iter());

            let (valid_neighbors, mut total_delta) = items
                .iter()
                .chain(neighbors)
                .map(|&(other_entity, other_pos)| {
                    let pos_delta = pos - other_pos;
                    let distance = pos_delta.length();
                    // Make sure that recip doesn't return infinity or very large values by adding a number.
                    let distance_recip = (distance + SAFETY_MARGIN).recip();
                    let valid = i32::from(other_entity != entity && distance < pref_dist);
                    (
                        valid,
                        valid as f32 * pos_delta * (distance_recip * (pref_dist - distance)),
                    )
                })
                .fold((0, Vec2::ZERO), |acc, x| (acc.0 + x.0, acc.1 + x.1));

            let jitter_remove_add = 3;
            total_delta /= (valid_neighbors + jitter_remove_add) as f32 * 0.5;

            if let Some(flow) = flow_field.get(nav_grid.pos_to_index(pos + total_delta)) {
                if *flow != Flow::None {
                    translation.translation.x += total_delta.x;
                    translation.translation.y += total_delta.y;
                    velocity.0 += total_delta / DELTA_TIME;
                }
            }
        }
    });
    stats.add("spatial_reset", reset_elapsed);
    stats.add("spatial_insert", insert_elapsed - reset_elapsed);
    stats.add("avoidance", start.elapsed() - insert_elapsed);
}

const SPATIAL_CELL_SIZE: f32 = PREFERRED_DISTANCE;
const SPATIAL_CELL_SIZE_INV: f32 = 1.0 / SPATIAL_CELL_SIZE;

#[derive(Debug, Clone, Default, Resource)]
pub struct SpatialStructure {
    level_size: f32,
    pub grid: HashMap<(i32, i32), Vec<(Entity, Vec2)>>,
}

const DEFAULT_CELL_CAPACITY: usize = 16;

impl SpatialStructure {
    pub fn new(level_size: f32) -> Self {
        Self {
            level_size,
            grid: HashMap::new(),
        }
    }

    pub fn reset(&mut self) {
        self.grid.clear();
    }

    pub fn insert(&mut self, (entity, pos): (Entity, Vec2)) {
        let cell = self.pos_to_cell(pos);
        let list = self
            .grid
            .entry(cell)
            .or_insert_with(|| Vec::with_capacity(DEFAULT_CELL_CAPACITY));

        if list.len() < 100 {
            list.push((entity, pos));
        }
    }

    pub fn neighbors(&self, cell: (i32, i32)) -> [Option<&Vec<(Entity, Vec2)>>; 8] {
        let (x, y) = cell;
        [
            self.grid.get(&(x - 1, y + 1)),
            self.grid.get(&(x, y + 1)),
            self.grid.get(&(x + 1, y + 1)),
            self.grid.get(&(x - 1, y)),
            self.grid.get(&(x + 1, y)),
            self.grid.get(&(x - 1, y - 1)),
            self.grid.get(&(x, y - 1)),
            self.grid.get(&(x + 1, y - 1)),
        ]
    }

    fn pos_to_cell(&self, pos: Vec2) -> (i32, i32) {
        let pos = pos.clamp(Vec2::ZERO, Vec2::splat(self.level_size));
        (
            (pos.x * SPATIAL_CELL_SIZE_INV) as i32,
            (pos.y * SPATIAL_CELL_SIZE_INV) as i32,
        )
    }
}

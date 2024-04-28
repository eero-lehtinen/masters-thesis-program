use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use crate::level::*;

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(level: Res<Level>, mut commands: Commands) {
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

    spatial
        .grid
        .iter()
        .enumerate()
        .filter(|(_, items)| !items.is_empty())
        .for_each(|(cell, items)| {
            let Some(neighbors) = spatial.get(cell) else {
                return;
            };
            for &(entity, pos) in items {
                let Ok((_, mut translation, mut velocity)) = enemy_q.get_mut(entity) else {
                    continue;
                };

                let (valid_neighbors, mut total_delta) = neighbors
                    .iter()
                    .flat_map(|v| v.iter())
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
    size: usize,
    pub grid: Vec<Vec<(Entity, Vec2)>>,
}

const DEFAULT_CELL_CAPACITY: usize = 16;

impl SpatialStructure {
    pub fn new(level_size: f32) -> Self {
        let size = (level_size * SPATIAL_CELL_SIZE_INV + 2.) as usize;
        Self {
            level_size,
            size,
            grid: vec![Vec::with_capacity(DEFAULT_CELL_CAPACITY); size * size],
        }
    }

    pub fn reset(&mut self) {
        self.grid.iter_mut().for_each(Vec::clear);
    }

    pub fn insert(&mut self, (entity, pos): (Entity, Vec2)) {
        let cell = self.pos_to_cell(pos);
        let a = unsafe { self.grid.get_unchecked_mut(cell) };
        if a.len() < 100 {
            a.push((entity, pos));
        }
    }

    pub fn get(&self, cell: usize) -> Option<[&[(Entity, Vec2)]; 9]> {
        if cell <= self.size || cell + self.size >= self.grid.len() - 1 {
            return None;
        }
        let up_pos = cell - self.size;
        let down_pos = cell + self.size;
        unsafe {
            Some([
                self.grid.get_unchecked(up_pos - 1).as_slice(),
                self.grid.get_unchecked(up_pos).as_slice(),
                self.grid.get_unchecked(up_pos + 1).as_slice(),
                self.grid.get_unchecked(cell - 1).as_slice(),
                self.grid.get_unchecked(cell).as_slice(),
                self.grid.get_unchecked(cell + 1).as_slice(),
                self.grid.get_unchecked(down_pos - 1).as_slice(),
                self.grid.get_unchecked(down_pos).as_slice(),
                self.grid.get_unchecked(down_pos + 1).as_slice(),
            ])
        }
    }

    fn pos_to_cell(&self, pos: Vec2) -> usize {
        let pos = pos.clamp(Vec2::ZERO, Vec2::splat(self.level_size));
        (pos.x * SPATIAL_CELL_SIZE_INV) as usize
            + 1
            + ((pos.y * SPATIAL_CELL_SIZE_INV) as usize + 1) * self.size
    }

    #[allow(dead_code)]
    fn print_stats(&self) {
        let mut total = 0;
        let mut avg_len = 0.0;
        let mut longest: usize = 0;
        let mut non_empties = 0;
        for v in self.grid.iter() {
            avg_len += v.len() as f32;
            if v.len() > longest {
                longest = v.len();
            }
            total += v.len();
            non_empties += i32::from(!v.is_empty());
        }
        avg_len /= non_empties as f32;
        info!(
            "spatial grid vec non empty cells: {}, avg len: {}, longest: {}, total: {}",
            non_empties, avg_len, longest, total
        );
    }
}

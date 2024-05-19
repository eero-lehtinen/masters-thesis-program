use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

#[cfg(feature = "parallel")]
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::simulation::movement;
use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use crate::level::*;

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(level: Res<Level>, mut commands: Commands) {
    println!("USING: spatial array");
    commands.insert_resource(SpatialStructure::new(level.size));
}

pub fn movement(world: &mut World) {
    let start = Instant::now();

    movement::move_with_flow_field(world);

    let mut system_state: SystemState<(
        Query<(Entity, &mut Transform, &mut Velocity), With<Enemy>>,
        Res<NavGrid>,
        Res<FlowField>,
        ResMut<SpatialStructure>,
        ResMut<Statistics>,
    )> = SystemState::new(world);
    let (enemy_q, nav_grid, flow_field, mut spatial, mut stats) = system_state.get_mut(world);

    spatial.reset();

    cfg_if::cfg_if! {
        if #[cfg(feature = "flocking_alignment")] {
            enemy_q
                .iter()
                .for_each(|(entity, tr, vel)| spatial.insert((entity, tr.translation.truncate(), vel.0.normalize_or_zero())));
        } else {
            enemy_q
                .iter()
                .for_each(|(entity, tr, _)| spatial.insert((entity, tr.translation.truncate())));
        }
    }

    stats.add("insert", start.elapsed());

    let pref_dist = PREFERRED_DISTANCE;

    #[cfg(not(feature = "parallel"))]
    let iter = spatial.grid.iter();
    #[cfg(feature = "parallel")]
    let iter = spatial.grid.par_iter();

    iter.enumerate()
        .filter(|(_, items)| !items.is_empty())
        .for_each(|(cell, items)| {
            let Some(neighbors) = spatial.get(cell) else {
                return;
            };
            for &(entity, pos, _) in items {
                let Ok((_, mut translation, mut velocity)) =
                    (unsafe { enemy_q.get_unchecked(entity) })
                else {
                    continue;
                };

                #[cfg(not(feature = "distance_func2"))]
                let total_delta = {
                    let mut total_force = Vec2::ZERO;
                    let mut valid_neighbors = 0;
                    for &(other_entity, other_pos, _) in neighbors.iter().flat_map(|v| v.iter()) {
                        if other_entity == entity {
                            continue;
                        }
                        let diff = pos - other_pos;
                        let distance = diff.length();
                        if distance < pref_dist {
                            let magnitude = pref_dist - distance;
                            let direction = 1. / distance * diff;
                            total_force += magnitude * direction;
                            valid_neighbors += 1;
                        }
                    }
                    total_force /= (valid_neighbors + 3) as f32;
                    total_force
                };

                #[cfg(feature = "distance_func2")]
                let total_delta = {
                    cfg_if::cfg_if!{
                        if #[cfg(all(feature = "branchless", feature = "floatneighbors", feature = "flocking_alignment"))] {
                            let (valid_neighbors, mut total_delta, total_dir) = neighbors
                                .iter()
                                .flat_map(|v| v.iter())
                                .map(|&(other_entity, other_pos, dir)| {
                                    let pos_delta = pos - other_pos;
                                    let distance = pos_delta.length();
                                    let magnitude = (pref_dist - distance).powi(2);
                                    let direction = 1. / (distance + SAFETY_MARGIN) * pos_delta;
                                    let force = magnitude * direction;
                                    let valid =
                                        f32::from(other_entity != entity && distance < pref_dist);
                                    (valid, valid * force, valid * dir, valid * distance)
                                })
                                .fold((0., Vec2::ZERO, Vec2::ZERO), |acc, x| (acc.0 + x.0, acc.1 + x.1, acc.2 + x.2));
                            total_delta /= valid_neighbors;
                            if let Some(dir) = total_dir.try_normalize() {
                                total_delta = dir * 0.4 * total_delta.length() + total_delta * 0.6;
                            }
                            total_delta * 2.
                        } else if #[cfg(all(feature = "branchless", feature = "floatneighbors"))] {
                            let (valid_neighbors, mut total_delta) = neighbors
                                .iter()
                                .flat_map(|v| v.iter())
                                .map(|&(other_entity, other_pos, _)| {
                                    let pos_delta = pos - other_pos;
                                    let distance = pos_delta.length();
                                    let magnitude = (pref_dist - distance).powi(2);
                                    let direction = 1. / (distance + SAFETY_MARGIN) * pos_delta;
                                    let force = magnitude * direction;
                                    let valid =
                                        f32::from(other_entity != entity && distance < pref_dist);
                                    (valid, valid * force)
                                })
                                .fold((0., Vec2::ZERO), |acc, x| (acc.0 + x.0, acc.1 + x.1));
                            total_delta /= valid_neighbors;
                            total_delta * 2.
                        } else if #[cfg(feature = "branchless")] {
                             let (valid_neighbors, mut total_delta) = neighbors
                                .iter()
                                .flat_map(|v| v.iter())
                                .map(|&(other_entity, other_pos, _)| {
                                    let pos_delta = pos - other_pos;
                                    let distance = pos_delta.length();
                                    let magnitude = (pref_dist - distance).powi(2);
                                    let direction = 1. / (distance + SAFETY_MARGIN) * pos_delta;
                                    let force = magnitude * direction;
                                    let valid =
                                        i32::from(other_entity != entity && distance < pref_dist);
                                    (valid, valid as f32 * force)
                                })
                                .fold((0, Vec2::ZERO), |acc, x| (acc.0 + x.0, acc.1 + x.1));
                            total_delta /= valid_neighbors as f32;
                            total_delta * 2.
                        } else {
                            let mut total_force = Vec2::ZERO;
                            let mut valid_neighbors = 0;
                            for &(other_entity, other_pos, _) in neighbors.iter().flat_map(|v| v.iter()) {
                                if other_entity == entity {
                                    continue;
                                }
                                let diff = pos - other_pos;
                                let distance = diff.length();
                                if distance < pref_dist {
                                    let magnitude = (pref_dist - distance).powi(2);
                                    let direction = 1. / distance * diff;
                                    total_force += magnitude * direction;
                                    valid_neighbors += 1;
                                }
                            }
                            total_force /= valid_neighbors as f32;
                            total_force * 2.
                        }
                    }
                };

                if let Some(flow) = flow_field.get(nav_grid.pos_to_index(pos + total_delta)) {
                    if *flow != Flow::None {
                        translation.translation.x += total_delta.x;
                        translation.translation.y += total_delta.y;
                        velocity.0 += total_delta / DELTA_TIME;
                    }
                }
            }
        });

    stats.add("movement", start.elapsed());
}

const SPATIAL_CELL_SIZE: f32 = PREFERRED_DISTANCE;
const SPATIAL_CELL_SIZE_INV: f32 = 1.0 / SPATIAL_CELL_SIZE;

#[derive(Debug, Clone, Default, Resource)]
pub struct SpatialStructure {
    level_size: f32,
    size: usize,
    #[cfg(not(feature = "flocking_alignment"))]
    pub grid: Vec<Vec<(Entity, Vec2, ())>>,
    #[cfg(feature = "flocking_alignment")]
    pub grid: Vec<Vec<(Entity, Vec2, Vec2)>>,
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
        self.grid.iter_mut().for_each(|a| a.clear());
    }

    cfg_if::cfg_if! {
        if #[cfg(not(feature = "flocking_alignment"))] {
            pub fn insert(&mut self, (entity, pos): (Entity, Vec2)) {
                let cell = self.pos_to_cell(pos);
                let a = unsafe { self.grid.get_unchecked_mut(cell) };
                if a.len() < 100 {
                    a.push((entity, pos, ()));
                }
            }

            pub fn get(&self, cell: usize) -> Option<[&[(Entity, Vec2, ())]; 9]> {
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
        } else {
            pub fn insert(&mut self, (entity, pos, dir): (Entity, Vec2, Vec2)) {
                let cell = self.pos_to_cell(pos);
                let a = unsafe { self.grid.get_unchecked_mut(cell) };
                if a.len() < 100 {
                    a.push((entity, pos, dir));
                }
            }

            pub fn get(&self, cell: usize) -> Option<[&[(Entity, Vec2, Vec2)]; 9]> {
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
        }

    }

    fn pos_to_cell(&self, pos: Vec2) -> usize {
        let pos = pos.clamp(Vec2::ZERO, Vec2::splat(self.level_size));
        (pos.x * SPATIAL_CELL_SIZE_INV) as usize
            + 1
            + ((pos.y * SPATIAL_CELL_SIZE_INV) as usize + 1) * self.size
    }
}

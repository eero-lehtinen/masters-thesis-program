use bevy::{prelude::*, utils::Instant};

use crate::{
    utils::{Easing, Velocity},
    DELTA_TIME,
};

use super::{
    level::{Enemy, Level, ENEMY_RADIUS},
    navigation::{Flow, FlowField, NavGrid},
    SimulationSet,
};

pub struct LocalAvoidancePlugin;

impl Plugin for LocalAvoidancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGridHash>().add_systems(
            PreUpdate,
            (make_spatial, keep_distance_to_others, reset_spatial_grid)
                .chain()
                .in_set(SimulationSet::LocalAvoidance),
        );
    }
}

const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 1.5;
const SAFETY_MARGIN: f32 = 0.0001;

const LOG_LOCAL_AVOIDANCE_DIAG: bool = false;

pub fn make_spatial(
    enemy_q: Query<(Entity, &Transform), With<Enemy>>,
    mut spatial: ResMut<SpatialGridHash>,
    level: Res<Level>,
) {
    spatial.reset(level.size);
    enemy_q.for_each(|(entity, tr)| spatial.insert((entity, tr.translation.truncate())));
    spatial.make_dirty();
}

pub fn keep_distance_to_others(
    mut enemy_q: Query<(&mut Transform, &mut Velocity), With<Enemy>>,
    nav_grid: Res<NavGrid>,
    flow_field: Res<FlowField>,
    spatial: Res<SpatialGridHash>,
    mut avg: Local<f32>,
    mut log_counter: Local<u32>,
) {
    let start = Instant::now();

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
                // SAFETY:
                // Each entity is only once in the spatial grid, so we don't get mutable aliasing in parallel iteration
                // so get_component_unchecked_mut is safe.
                // Also entities can't disappear while iterating so unwrap_unchecked is quaranteed to succeed.
                let Ok((mut translation, mut velocity)) = enemy_q.get_mut(entity) else {
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

                // TODO: Could move to the outermost possible edge instead to reduce jumping
                if let Some((_, flow)) = flow_field.get(nav_grid.pos_to_index(pos + total_delta)) {
                    if *flow != Flow::None {
                        translation.translation.x += total_delta.x;
                        translation.translation.y += total_delta.y;
                        velocity.0 += total_delta / DELTA_TIME;
                    }
                }
            }
        });
    if LOG_LOCAL_AVOIDANCE_DIAG {
        let end = start.elapsed();
        *avg = avg.lerp(end.as_secs_f32() * 1000., 0.1);
        *log_counter += 1;
        if *log_counter % 60 == 0 {
            info!("total: {:?}, avg: {:?}", end, *avg);
        }
        // spatial.print_stats();
    }
}

// This is run in Update to save time in fixed update
pub fn reset_spatial_grid(mut spatial: ResMut<SpatialGridHash>, level: Res<Level>) {
    spatial.reset(level.size);
}

const SPATIAL_CELL_SIZE: f32 = PREFERRED_DISTANCE;
const SPATIAL_CELL_SIZE_INV: f32 = 1.0 / SPATIAL_CELL_SIZE;

#[derive(Debug, Clone, Default, Resource)]
pub struct SpatialGridHash {
    level_size: f32,
    size: usize,
    pub grid: Vec<Vec<(Entity, Vec2)>>,
    reset: bool,
}

const DEFAULT_CELL_CAPACITY: usize = 16;

impl SpatialGridHash {
    pub fn reset(&mut self, level_size: f32) {
        #[allow(clippy::float_cmp)]
        if self.reset && self.level_size == level_size {
            return;
        }
        self.level_size = level_size;
        self.grid.iter_mut().for_each(Vec::clear);
        self.size = (level_size * SPATIAL_CELL_SIZE_INV + 2.) as usize;
        self.grid.resize(
            self.size * self.size,
            Vec::with_capacity(DEFAULT_CELL_CAPACITY),
        );
        self.reset = true;
    }

    pub fn insert(&mut self, (entity, pos): (Entity, Vec2)) {
        let cell = self.pos_to_cell(pos);
        let a = unsafe { self.grid.get_unchecked_mut(cell) };
        if a.len() < 100 {
            a.push((entity, pos));
        }
    }

    // Should be called after all inserts are done
    pub fn make_dirty(&mut self) {
        self.reset = false;
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

    #[allow(dead_code)]
    pub fn get_iter_with_distance_override(
        &self,
        pos: Vec2,
        distance: f32,
    ) -> impl Iterator<Item = &(Entity, Vec2)> {
        let cell = self.pos_to_cell(pos);
        let d = (distance * SPATIAL_CELL_SIZE_INV).ceil() as i32;
        let cells = (-d..=d)
            .flat_map(move |x| (-d..=d).map(move |y| cell as i32 + x + y * self.size as i32));
        cells
            .filter_map(move |cell| self.grid.get(cell as usize))
            .flatten()
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

use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};
use itertools::Itertools;
use kiddo::float::{distance::SquaredEuclidean, kdtree::KdTree};

use crate::simulation::spawning::MAX_ENEMIES;
use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(mut commands: Commands) {
    println!("USING: kiddo kdtree");
    commands.insert_resource(SpatialStructure::new());
}

pub fn keep_distance_to_others(world: &mut World) {
    let mut system_state: SystemState<(
        Query<(&mut Transform, &mut Velocity), With<Enemy>>,
        Res<NavGrid>,
        Res<FlowField>,
        ResMut<SpatialStructure>,
        ResMut<Statistics>,
    )> = SystemState::new(world);
    let (mut enemy_q, nav_grid, flow_field, mut spatial, mut stats) = system_state.get_mut(world);

    let start = Instant::now();
    let reset_elapsed = start.elapsed();

    spatial.tree = KdTree::with_capacity(enemy_q.iter().len());
    let positions = enemy_q
        .iter()
        .enumerate()
        .map(|(i, (tr, _))| {
            let pos = tr.translation.truncate();
            spatial.tree.add(&pos.to_array(), i);
            pos
        })
        .collect_vec();

    let insert_elapsed = start.elapsed();

    let pref_dist = PREFERRED_DISTANCE;

    for (mut translation, mut velocity) in enemy_q.iter_mut() {
        let pos = translation.translation.truncate();

        let (valid_neighbors, mut total_delta) = spatial
            .tree
            .within_unsorted_iter::<SquaredEuclidean>(&pos.to_array(), pref_dist.powi(2))
            .map(|n| {
                let other_pos = positions[n.item];
                let pos_delta = pos - other_pos;
                let distance = n.distance.sqrt();
                let distance_recip = (distance + SAFETY_MARGIN).recip();
                let valid = i32::from(pos_delta != Vec2::ZERO);
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

    stats.add("spatial_reset", reset_elapsed);
    stats.add("spatial_insert", insert_elapsed - reset_elapsed);
    stats.add("avoidance", start.elapsed() - insert_elapsed);
}

#[derive(Resource)]
pub struct SpatialStructure {
    tree: KdTree<f32, usize, 2, 32, u16>,
}

impl SpatialStructure {
    pub fn new() -> Self {
        SpatialStructure {
            tree: KdTree::new(),
        }
    }
}

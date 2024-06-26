use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use crate::simulation::movement;
use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(mut commands: Commands) {
    info!("USING: rstar");
    commands.insert_resource(SpatialStructure::new());
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
    let (mut enemy_q, nav_grid, flow_field, mut spatial, mut stats) = system_state.get_mut(world);

    let positions = enemy_q
        .iter()
        .map(|(_, tr, _)| tr.translation.truncate().to_array())
        .collect::<Vec<_>>();
    spatial.tree = RTree::bulk_load(positions);

    let pref_dist = PREFERRED_DISTANCE;

    for (_, mut translation, mut velocity) in enemy_q.iter_mut() {
        let pos = translation.translation.truncate();

        let (valid_neighbors, mut total_delta) = spatial
            .tree
            .locate_within_distance(pos.to_array(), pref_dist.powi(2))
            .map(|other_pos| {
                let pos_delta = pos - Vec2::from(*other_pos);
                let distance = pos_delta.length();
                let magnitude = (pref_dist - distance).powi(2);
                let direction = 1. / (distance + SAFETY_MARGIN) * pos_delta;
                let force = magnitude * direction;
                let valid = f32::from(distance != 0.);
                (valid, valid * force)
            })
            .fold((0., Vec2::ZERO), |acc, x| (acc.0 + x.0, acc.1 + x.1));

        total_delta /= valid_neighbors;
        total_delta *= 2.;

        if let Some(flow) = flow_field.get(nav_grid.pos_to_index(pos + total_delta)) {
            if *flow != Flow::None {
                translation.translation.x += total_delta.x;
                translation.translation.y += total_delta.y;
                velocity.0 += total_delta / DELTA_TIME;
            }
        }
    }

    stats.add("movement", start.elapsed());
}

use rstar::RTree;

#[derive(Debug, Clone, Resource)]
pub struct SpatialStructure {
    tree: RTree<[f32; 2]>,
}

impl SpatialStructure {
    pub fn new() -> Self {
        SpatialStructure { tree: RTree::new() }
    }
}

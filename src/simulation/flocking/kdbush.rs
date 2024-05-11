use bevy::{ecs::system::SystemState, prelude::*, utils::Instant};

use crate::simulation::spawning::MAX_ENEMIES;
use crate::{statistics::Statistics, utils::Velocity, DELTA_TIME};

use crate::simulation::{
    navigation::{Flow, FlowField, NavGrid},
    spawning::Enemy,
};

use super::{PREFERRED_DISTANCE, SAFETY_MARGIN};

pub fn init(mut commands: Commands) {
    println!("USING: kdbush");
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

    spatial.tree = KDBush::new(MAX_ENEMIES as usize, 32);
    let positions = enemy_q
        .iter()
        .enumerate()
        .map(|(i, (tr, _))| {
            let pos = tr.translation.truncate();
            spatial.tree.add_point(i, pos.x as f64, pos.y as f64);
            pos
        })
        .collect::<Vec<_>>();
    spatial.tree.build_index();

    let pref_dist = PREFERRED_DISTANCE;

    for (mut translation, mut velocity) in enemy_q.iter_mut() {
        let pos = translation.translation.truncate();

        let mut valid_neighbors = 0.;
        let mut total_delta = Vec2::ZERO;
        spatial
            .tree
            .within(pos.x as f64, pos.y as f64, pref_dist as f64, |id| {
                let other_pos = positions[id];
                let pos_delta = pos - other_pos;
                let distance = pos_delta.length();
                let magnitude = (pref_dist - distance).powi(2);
                let direction = 1. / (distance + SAFETY_MARGIN) * pos_delta;
                let force = magnitude * direction;
                let valid = f32::from(distance != 0.);
                valid_neighbors += valid;
                total_delta += valid * force;
            });
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

    stats.add("flocking", start.elapsed());
}

use kdbush::KDBush;

#[derive(Resource)]
pub struct SpatialStructure {
    tree: KDBush,
}

impl SpatialStructure {
    pub fn new() -> Self {
        SpatialStructure {
            tree: KDBush::new(MAX_ENEMIES as usize, 32),
        }
    }
}

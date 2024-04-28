use bevy::prelude::*;

mod array;
use array::{init, keep_distance_to_others};

// mod kdtree;
// use kdtree::{init, keep_distance_to_others};

use super::{spawning::ENEMY_RADIUS, SimulationSet};

pub struct FlockingPlugin;

impl Plugin for FlockingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init).add_systems(
            PreUpdate,
            keep_distance_to_others.in_set(SimulationSet::LocalAvoidance),
        );
    }
}

const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 1.5;
const SAFETY_MARGIN: f32 = 0.0001;

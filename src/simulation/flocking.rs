use bevy::prelude::*;

// cfg_if::cfg_if! {
// if #[cfg(feature = "spatial_hash")] {
//     mod hash;
//     use hash::{init, keep_distance_to_others};
// } else if #[cfg(feature = "spatial_kdtree")] {
//     mod kdtree;
//     use kdtree::{init, keep_distance_to_others};
// } else if #[cfg(feature = "spatial_kdtree_kiddo")] {
//     mod kdtree_kiddo;
//     use kdtree_kiddo::{init, keep_distance_to_others};
// } else if #[cfg(feature = "spatial_kdbush")] {
//     mod kdbush;
//     use kdbush::{init, keep_distance_to_others};
// } else if #[cfg(feature = "spatial_rstar")] {
//     mod rstar;
//     use rstar::{init, keep_distance_to_others};
// } else {
mod array;
use array::{init, keep_distance_to_others};
// }
// }

use super::{spawning::ENEMY_RADIUS, SimulationSet};

pub struct FlockingPlugin;

impl Plugin for FlockingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init).add_systems(
            PreUpdate,
            keep_distance_to_others.in_set(SimulationSet::Flocking),
        );
    }
}

#[cfg(not(feature = "distance_func2"))]
const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 2.;
#[cfg(feature = "distance_func2")]
const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 2.2;

const SAFETY_MARGIN: f32 = 0.000001;

use bevy::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(any(feature = "spatial_hash", feature = "spatial_hash_std"))] {
        mod hash;
        use hash::{init, movement};
    } else if #[cfg(feature = "spatial_kdtree")] {
        mod kdtree;
        use kdtree::{init, movement};
    } else if #[cfg(feature = "spatial_kdtree_kiddo")] {
        mod kdtree_kiddo;
        use kdtree_kiddo::{init, movement};
    } else if #[cfg(feature = "spatial_kdbush")] {
        mod kdbush;
        use kdbush::{init, movement};
    } else if #[cfg(feature = "spatial_rstar")] {
        mod rstar;
        use rstar::{init, movement};
    } else {
        mod array;
        use array::{init, movement};
    }
}

use super::{spawning::ENEMY_RADIUS, SimulationSet};

pub struct FlockingPlugin;

impl Plugin for FlockingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init)
            .add_systems(PreUpdate, movement.in_set(SimulationSet::Movement));
    }
}

#[cfg(not(feature = "distance_func2"))]
const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 2.;
#[cfg(feature = "distance_func2")]
const PREFERRED_DISTANCE: f32 = ENEMY_RADIUS * 2.2;

const SAFETY_MARGIN: f32 = 0.000001;

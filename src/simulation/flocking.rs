use bevy::prelude::*;

#[cfg(spatial_array)]
mod array;
#[cfg(spatial_array)]
use array::{init, keep_distance_to_others};

#[cfg(spatial_kdtree)]
mod kdtree;
#[cfg(spatial_kdtree)]
use kdtree::{init, keep_distance_to_others};

#[cfg(spatial_kdbush)]
mod kdbush;
#[cfg(spatial_kdbush)]
use kdbush::{init, keep_distance_to_others};

#[cfg(spatial_quadtree)]
mod quadtree;
#[cfg(spatial_quadtree)]
use quadtree::{init, keep_distance_to_others};

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

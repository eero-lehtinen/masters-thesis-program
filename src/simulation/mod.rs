use bevy::prelude::*;

use self::{
    local_avoidance::LocalAvoidancePlugin, movement::MovementPlugin, navigation::NavigationPlugin,
    spawning::SpawningPlugin,
};

mod collision;
mod local_avoidance;
mod movement;

#[cfg(feature = "navigation1")]
pub mod navigation;
#[cfg(feature = "navigation2")]
mod navigation2;
#[cfg(feature = "navigation2")]
pub mod navigation {
    pub use super::navigation2::*;
}

mod rng;
pub mod spawning;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MovementPlugin,
            LocalAvoidancePlugin,
            NavigationPlugin,
            SpawningPlugin,
            // CollisionPlugin,
        ))
        .configure_sets(
            PreUpdate,
            (
                SimulationSet::Despawn,
                SimulationSet::Spawn,
                SimulationSet::Flush,
                SimulationSet::GenNavigation,
                SimulationSet::Move,
                SimulationSet::LocalAvoidance,
                SimulationSet::ApplyColliders,
            )
                .chain(),
        )
        .add_systems(PreUpdate, apply_deferred.in_set(SimulationSet::Flush));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum SimulationSet {
    Despawn,
    Spawn,
    Flush,
    GenNavigation,
    Move,
    LocalAvoidance,
    ApplyColliders,
}

use bevy::prelude::*;

use self::{
    local_avoidance::LocalAvoidancePlugin, movement::MovementPlugin, navigation::NavigationPlugin,
    spawning::SpawningPlugin,
};

mod collision;
mod local_avoidance;
mod movement;
pub mod navigation;
mod rng;
pub mod spawning;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            NavigationPlugin,
            MovementPlugin,
            LocalAvoidancePlugin,
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

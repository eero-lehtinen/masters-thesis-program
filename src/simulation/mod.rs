use bevy::prelude::*;

use self::{
    collision::CollisionPlugin, level::LevelPlugin, local_avoidance::LocalAvoidancePlugin,
    movement::MovementPlugin, navigation::NavigationPlugin,
};

mod collision;
pub mod level;
mod local_avoidance;
mod movement;
pub mod navigation;
mod rng;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            NavigationPlugin,
            MovementPlugin,
            LocalAvoidancePlugin,
            // CollisionPlugin,
            LevelPlugin,
        ))
        .edit_schedule(PreUpdate, |s| {
            s.configure_sets(
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
            );
        })
        .edit_schedule(Startup, |s| {
            s.configure_sets((SimulationStartupSet::Spawn, SimulationStartupSet::Flush).chain());
        })
        .add_systems(PreUpdate, apply_deferred.in_set(SimulationSet::Flush))
        .add_systems(Startup, apply_deferred.in_set(SimulationStartupSet::Flush));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum SimulationSet {
    Spawn,
    Flush,
    GenNavigation,
    Move,
    Despawn,
    LocalAvoidance,
    ApplyColliders,
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum SimulationStartupSet {
    Spawn,
    Flush,
}

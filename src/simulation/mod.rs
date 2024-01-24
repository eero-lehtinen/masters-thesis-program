use bevy::prelude::*;

use self::{collision::CollisionPlugin, level::LevelPlugin, navigation::NavigationPlugin};

mod collision;
pub mod level;
mod navigation;
mod rng;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((NavigationPlugin, CollisionPlugin, LevelPlugin))
            .edit_schedule(PreUpdate, |s| {
                s.configure_sets(
                    (
                        SimulationSet::Spawn,
                        SimulationSet::Flush,
                        SimulationSet::GenNavigation,
                        SimulationSet::Move,
                        SimulationSet::ApplyColliders,
                    )
                        .chain(),
                );
            })
            .add_systems(PreUpdate, apply_deferred.in_set(SimulationSet::Flush));
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub enum SimulationSet {
    Spawn,
    Flush,
    GenNavigation,
    Move,
    ApplyColliders,
}

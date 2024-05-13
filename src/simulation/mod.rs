use bevy::prelude::*;

use self::{flocking::FlockingPlugin, navigation::NavigationPlugin, spawning::SpawningPlugin};

mod collision;
mod flocking;
mod movement;

// #[cfg(navigation1)]
pub mod navigation;
// #[cfg(navigation2)]
// mod navigation2;
// #[cfg(navigation2)]
// pub mod navigation {
//     pub use super::navigation2::*;
// }

mod rng;
pub mod spawning;

pub struct SimulationPlugin {
    pub update_nav: bool,
}

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FlockingPlugin,
            NavigationPlugin {
                update: self.update_nav,
            },
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
                SimulationSet::Movement,
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
    Movement,
    ApplyColliders,
}

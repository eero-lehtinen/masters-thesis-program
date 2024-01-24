#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use bevy_framepace::FramepaceSettings;
use visualization::VisualizationPlugin;

use crate::simulation::SimulationPlugin;

pub mod simulation;
pub mod utils;
pub mod visualization;

/// How often the simulation is updated when visualizing.
/// Benchmarks are ran as fast as possible.
const FRAME_RATE: i32 = 60;
const DELTA_TIME_F64: f64 = 1.0 / FRAME_RATE as f64;
const DELTA_TIME: f32 = 1.0 / FRAME_RATE as f32;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            SimulationPlugin,
            VisualizationPlugin,
            bevy_framepace::FramepacePlugin,
        ))
        .insert_resource(FramepaceSettings {
            limiter: bevy_framepace::Limiter::from_framerate(FRAME_RATE as f64),
        })
        .run();
}

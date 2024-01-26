#![allow(clippy::type_complexity)]

use bevy::{app::AppExit, prelude::*, window::WindowResolution};
use framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use statistics::StatisticsPlugin;
use visualization::VisualizationPlugin;

use crate::simulation::SimulationPlugin;

mod framepace;
pub mod simulation;
pub mod statistics;
pub mod utils;
pub mod visualization;

/// How often the simulation is updated when visualizing.
/// Benchmarks are ran as fast as possible.
const FRAME_RATE: i32 = 60;
const DELTA_TIME: f32 = 1.0 / FRAME_RATE as f32;

const BENCHMARK_TICKS: u32 = 1000;

fn main() {
    if cfg!(not(feature = "bench")) {
        App::new()
            .add_plugins((
                DefaultPlugins.set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(700., 700.),
                        ..default()
                    }),
                    ..default()
                }),
                SimulationPlugin,
                VisualizationPlugin,
                FramepacePlugin,
                StatisticsPlugin,
            ))
            .insert_resource(FramepaceSettings {
                limiter: Limiter::from_framerate(FRAME_RATE as f64),
            })
            .init_resource::<Ticks>()
            .add_systems(PostUpdate, update_tick)
            .run();
    } else {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, SimulationPlugin, StatisticsPlugin))
            .init_resource::<Ticks>()
            .add_systems(PostUpdate, (update_tick, exit_bench).chain());
        // bevy_mod_debugdump::print_schedule_graph(&mut app, PreUpdate);
        app.run();
    }
}

#[derive(Resource, Default)]
pub struct Ticks(pub u32);

fn update_tick(mut ticks: ResMut<Ticks>) {
    ticks.0 += 1;
}

fn exit_bench(mut exit: ResMut<Events<AppExit>>, ticks: Res<Ticks>) {
    if ticks.0 >= BENCHMARK_TICKS {
        exit.send(AppExit);
    }
}

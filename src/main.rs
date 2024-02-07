#![allow(clippy::type_complexity)]

use std::fs::File;

use bevy::{
    app::AppExit, core::FrameCount, prelude::*, time::TimePlugin, window::WindowResolution,
};
use editor::EditorPlugin;
use framepace::{FramepacePlugin, FramepaceSettings, Limiter};
use level::{Level, LevelPath, LevelPlugin};
use statistics::StatisticsPlugin;
use visualization::VisualizationPlugin;

use crate::simulation::SimulationPlugin;

use clap::{Parser, Subcommand};

mod editor;
mod framepace;
pub mod level;
pub mod simulation;
pub mod statistics;
pub mod utils;
pub mod visualization;

/// How often the simulation is updated when visualizing.
/// Benchmarks are ran as fast as possible.
const FRAME_RATE: i32 = 60;
const DELTA_TIME: f32 = 1.0 / FRAME_RATE as f32;

const BENCHMARK_TICKS: u32 = 1000;

#[derive(Parser)]
struct Cli {
    level: Option<String>,

    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Eq, PartialEq, Resource, Clone, Copy)]
enum Command {
    Viewer,
    Editor,
    Bench,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut app = App::new();

    let command = cli.command.unwrap_or(Command::Viewer);
    app.insert_resource(command);

    if command == Command::Bench {
        app.add_plugins((MinimalPlugins.build().disable::<TimePlugin>(),))
            .add_systems(First, exit_bench);
    } else {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(700., 700.),
                    present_mode: if command == Command::Editor {
                        bevy::window::PresentMode::AutoVsync
                    } else {
                        bevy::window::PresentMode::AutoNoVsync
                    },
                    ..default()
                }),
                ..default()
            }),
            VisualizationPlugin,
            FramepacePlugin,
        ));
    }

    if command == Command::Viewer {
        app.insert_resource(FramepaceSettings {
            limiter: Limiter::from_framerate(FRAME_RATE as f64),
        });
    }

    if command == Command::Editor {
        app.add_plugins(EditorPlugin);
    } else {
        app.add_plugins((SimulationPlugin, StatisticsPlugin));
    }

    app.add_plugins(LevelPlugin);
    if let Some(level_path) = cli.level {
        let file = File::open(&level_path)?;
        let level: Level = rmp_serde::from_read(file)?;
        app.insert_resource(level)
            .insert_resource(LevelPath(level_path));
    } else {
        app.insert_resource(Level::default());
    }

    app.run();

    Ok(())
}

fn exit_bench(mut exit: ResMut<Events<AppExit>>, frames: Res<FrameCount>) {
    if frames.0 >= BENCHMARK_TICKS {
        exit.send(AppExit);
    }
}

# Crowd Simulation in Games: Evaluation Program

## Introduction

This reposotory contains the evaluation program used for my Master's thesis "Crowd Simulation in Games".
It is used to compare the performances of different algorithms and libraries for crowd simulation.

## Requirements

Install Rust (preferably using [rustup](https://rustup.rs/)).

## Usage

```sh
# Run the simulator on level "3-Cathedral" with a visualizer
cargo run -r -- --update-nav --level 3-Cathedral viewer

# Run the simulator headlessly as a benchmark
cargo run -r -- --level 3-Cathedral benchmark

# Run the level editor
cargo run -r -- editor
```

In the viewer, move around by dragging the mouse and zoom in/out with the scroll wheel.
Left click to make agents follow the cursor.
Press `F` to toggle flow field arrows.

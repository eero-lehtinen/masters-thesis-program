# Crowd Simulation in Games: Evaluation Program

## Introduction

This reposotory contains the evaluation program used for my Master's thesis "Crowd Simulation in Games".
It is used to compare the performances of different algorithms and libraries for crowd simulation.

## Requirements

Install Rust (preferably using [rustup](https://rustup.rs/)).

## Usage

```sh
# Run the simulator on level "a" with a visualizer
cargo run -r -- --level a viewer

# Run the simulator headlessly as a benchmark for 1000 ticks
cargo run -r -- --level a bench --ticks 1000

# Run the level editor
cargo run -r -- editor
```


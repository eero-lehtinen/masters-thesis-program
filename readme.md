# Crowd Simulation in Games: Evaluation Program

## Introduction

This reposotory contains the evaluation program used for my Master's thesis "Crowd Simulation in Games".
It is used to compare the performances of different algorithms and libraries for crowd simulation.

## Requirements

Install Rust (preferably using [rustup](https://rustup.rs/)).

## Running Examples

```sh
# Run the simulator on level "a" with a visualizer (feature navigation1 or navigation2 is needed)
cargo run -r --features navigation1 -- --level a viewer

# Run the simulator headlessly as a benchmark for 1000 ticks
cargo run -r --features navigation1 -- --level a bench --ticks 1000

# Run the level editor
cargo run -r --features navigation1 -- editor
```


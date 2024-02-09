use crate::{
    level::{Level, LevelStartupSet, Target},
    statistics::Statistics,
    utils::{ToUsizeArr, ToVec2, Vertices},
};
use bevy::{
    ecs::system::SystemState,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::{HashSet, Instant},
};
use futures_lite::future;
use geo_types::Coordinate;
use itertools::Itertools;
use ndarray::Array2;
use offset_polygon::offset_polygon;
use std::{collections::VecDeque, f32::consts::SQRT_2, iter, sync::Arc, time::Duration};

use super::{spawning::ENEMY_RADIUS, SimulationSet};

pub const NAV_SCALE: f32 = ENEMY_RADIUS;
pub const NAV_SCALE_INV: f32 = 1. / NAV_SCALE;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlowField>()
            .insert_resource(RunInTask(true))
            .init_resource::<FlowFieldGenerate>()
            .add_systems(PreStartup, init_nav_grid.after(LevelStartupSet::Spawn))
            .add_systems(
                PreUpdate,
                (
                    generate_flow_field_system.run_if(resource_equals(RunInTask(false))),
                    (start_flow_field_generation_task, handle_flow_field_task)
                        .chain()
                        .run_if(resource_equals(RunInTask(true))),
                )
                    .in_set(SimulationSet::GenNavigation),
            );
    }
}

#[derive(Resource, PartialEq, Eq)]
struct RunInTask(bool);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct NavGrid(pub Arc<NavGridInner>);

#[derive(Default)]
pub struct NavGridInner {
    #[allow(dead_code)]
    size: f32,
    inflated_walls: Vec<Vertices>,
    walkable: Array2<bool>,
    /// Contains bitsets of directions that can be moved in from a given index
    pub grid: Array2<u8>,
}

fn init_nav_grid(mut commands: Commands, level: Res<Level>) {
    let nav_grid = NavGridInner::new(level.size, &level.walls);
    commands.insert_resource(NavGrid(Arc::new(nav_grid)));
}

pub type FlowFieldInner = Array2<(f32, Flow)>;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct FlowField(pub FlowFieldInner);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Flow {
    #[default]
    None,
    Source,
    /// Contains direction to move in
    LineOfSight(Vec2), // TODO: Test with only f32 (or u8) as angle to reduce memory usage (and increase cache locality)
    North,
    East,
    South,
    West,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

impl Flow {
    pub fn to_dir(self) -> Vec2 {
        match self {
            Self::None | Self::Source => Vec2::ZERO,
            Self::North => Vec2::Y,
            Self::NorthEast => Vec2::new(SQRT_2 / 2., SQRT_2 / 2.),
            Self::East => Vec2::X,
            Self::SouthEast => Vec2::new(SQRT_2 / 2., -SQRT_2 / 2.),
            Self::South => Vec2::NEG_Y,
            Self::SouthWest => Vec2::new(-SQRT_2 / 2., -SQRT_2 / 2.),
            Self::West => Vec2::NEG_X,
            Self::NorthWest => Vec2::new(-SQRT_2 / 2., SQRT_2 / 2.),
            Self::LineOfSight(v) => v,
        }
    }

    pub const fn distance(self) -> f32 {
        match self {
            Self::None | Self::Source | Self::LineOfSight(_) => 0.,
            Self::North | Self::East | Self::South | Self::West => 1.,
            Self::NorthEast | Self::SouthEast | Self::SouthWest | Self::NorthWest => SQRT_2,
        }
    }

    pub fn approx_mask(dir: Vec2) -> u8 {
        let mut mask = 0;
        if dir.x > 0. && dir.y > 0. {
            mask |= Self::East.mask() | Self::North.mask() | Self::NorthEast.mask();
        } else if dir.x < 0. && dir.y > 0. {
            mask |= Self::West.mask() | Self::North.mask() | Self::NorthWest.mask();
        } else if dir.x > 0. && dir.y < 0. {
            mask |= Self::East.mask() | Self::South.mask() | Self::SouthEast.mask();
        } else if dir.x < 0. && dir.y < 0. {
            mask |= Self::West.mask() | Self::South.mask() | Self::SouthWest.mask();
        }
        mask
    }

    #[inline]
    pub const fn mask(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Source => 0,
            Self::LineOfSight(_) => 0,
            Self::North => 0b0000_0001,
            Self::East => 0b0000_0010,
            Self::South => 0b0000_0100,
            Self::West => 0b0000_1000,
            Self::NorthEast => 0b0001_0000,
            Self::SouthEast => 0b0010_0000,
            Self::SouthWest => 0b0100_0000,
            Self::NorthWest => 0b1000_0000,
        }
    }

    const DIRECTIONALS: [Self; 8] = [
        Self::North,
        Self::East,
        Self::South,
        Self::West,
        Self::NorthEast,
        Self::SouthEast,
        Self::SouthWest,
        Self::NorthWest,
    ];
}

impl NavGridInner {
    pub fn new(size: f32, walls: &[Vertices]) -> Self {
        // Expand walls
        let walls = walls
            .iter()
            .filter_map(|w| inflate_polygon(w, ENEMY_RADIUS * 1.3))
            .collect::<Vec<_>>();

        let scale = NAV_SCALE;
        let scaled_size = (size / scale) as usize + 2;
        let mut walkable = Array2::from_elem((scaled_size, scaled_size), false);
        for x in 1..scaled_size - 1 {
            for y in 1..scaled_size - 1 {
                let pos = Self::index_to_pos_impl(Vec2::new(x as f32, y as f32));
                let mut w = true;
                for vertices in &walls {
                    if is_point_in_polygon(pos, vertices) {
                        w = false;
                        break;
                    }
                }
                walkable[[x, y]] = w;
            }
        }
        let mut grid = Array2::from_elem((scaled_size, scaled_size), 0);
        for x in 1..scaled_size - 1 {
            for y in 1..scaled_size - 1 {
                let mut bitset = 0;
                for (i, flow) in Flow::DIRECTIONALS.iter().enumerate() {
                    let [nx, ny] = neighbor_idx([x, y], *flow);
                    if i < 4 {
                        if walkable[[nx, ny]] {
                            bitset |= 1 << i;
                        }
                    } else {
                        // Disallow diagonals if either of the cardinal directions are blocked
                        if walkable[[nx, ny]] && walkable[[x, ny]] && walkable[[nx, y]] {
                            bitset |= 1 << i;
                        }
                    }
                }
                grid[[x, y]] = bitset;
            }
        }

        Self {
            size,
            inflated_walls: walls,
            walkable,
            grid,
        }
    }

    pub fn pos_to_index(&self, pos: Vec2) -> [usize; 2] {
        let pos = (pos * NAV_SCALE_INV + Vec2::ONE).floor();
        [pos.x as usize, pos.y as usize]
    }

    pub fn index_to_pos(index: [usize; 2]) -> Vec2 {
        Self::index_to_pos_impl(index.to_vec2())
    }

    fn index_to_pos_impl(index: Vec2) -> Vec2 {
        (index - 1.) * NAV_SCALE + NAV_SCALE * 0.5
    }

    pub const fn walkable(&self) -> &Array2<bool> {
        &self.walkable
    }

    pub fn inflated_walls(&self) -> &[Vertices] {
        &self.inflated_walls
    }

    fn raycast_walkable_dda(
        &self,
        start: [usize; 2],
        end: [usize; 2],
        // lines: &mut DebugLines,
    ) -> bool {
        let rel = end.to_vec2() - start.to_vec2();
        let steps = rel.x.abs().max(rel.y.abs());

        let delta = rel / steps;

        let mut cur = start.to_vec2();
        for _ in 0..(steps as usize + 1) {
            if !self.walkable[cur.round().to_usize_arr()] {
                return false;
            }
            cur += delta;
        }
        true
    }
}

/// Actually returns the "opposite" of the flow, this is used to find the neighbor
#[inline]
pub const fn neighbor_idx([x, y]: [usize; 2], flow: Flow) -> [usize; 2] {
    match flow {
        Flow::None | Flow::Source | Flow::LineOfSight(_) => {
            panic!("No neighbor for None, Source or LineOfSight")
        }
        Flow::North => [x, y - 1],
        Flow::East => [x - 1, y],
        Flow::South => [x, y + 1],
        Flow::West => [x + 1, y],
        Flow::NorthEast => [x - 1, y - 1],
        Flow::SouthEast => [x - 1, y + 1],
        Flow::SouthWest => [x + 1, y + 1],
        Flow::NorthWest => [x + 1, y - 1],
    }
}

fn find_valid_source(nav_grid: &NavGrid, pos: Vec2) -> [usize; 2] {
    let idx = nav_grid.pos_to_index(pos);
    if nav_grid.walkable()[idx] {
        return idx;
    }
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    queue.push_back(idx);
    visited.insert(idx);
    while let Some(idx) = queue.pop_front() {
        for flow in &Flow::DIRECTIONALS {
            let idx = neighbor_idx(idx, *flow);
            let Some(walkable) = nav_grid.walkable().get(idx) else {
                continue;
            };
            if *walkable {
                return idx;
            }
            if visited.contains(&idx) {
                continue;
            }
            queue.push_back(idx);
        }
    }
    // Shouldn't happen with valid levels, but just in case
    idx
}

#[inline(always)]
fn check_neighbor(
    flow: Flow,
    dist: f32,
    grid_val: u8,
    idx: [usize; 2],
    flow_field: &mut Array2<(f32, Flow)>,
    queue: &mut VecDeque<(f32, [usize; 2])>,
) {
    if grid_val & flow.mask() != 0 {
        let neigh_idx = neighbor_idx(idx, flow);
        let new_dist = dist + flow.distance();
        let f = &mut flow_field[neigh_idx];
        if f.0 > new_dist {
            *f = (new_dist, flow);
            queue.push_back((new_dist, neigh_idx));
        }
    }
}

#[inline(always)]
fn check_neighbor_raycast(
    flow: Flow,
    idx: [usize; 2],
    grid_val: u8,
    source: [usize; 2],
    nav_grid: &NavGridInner,
    flow_field: &mut Array2<(f32, Flow)>,
    queue: &mut VecDeque<(f32, [usize; 2], [usize; 2])>,
) {
    if grid_val & flow.mask() != 0 {
        let neigh_idx = neighbor_idx(idx, flow);
        let f = &mut flow_field[neigh_idx];
        if !matches!(f.1, Flow::LineOfSight(_) | Flow::Source) {
            let diff = neigh_idx.to_vec2() - source.to_vec2();
            let diff_length = diff.length();
            let new_dist = diff_length - 0.1;
            if new_dist < f.0 {
                let normalized = diff * diff_length.recip();
                let mask = Flow::approx_mask(normalized);
                if nav_grid.grid[neigh_idx] & mask == mask
                    && nav_grid.raycast_walkable_dda(source, neigh_idx)
                {
                    *f = (new_dist, Flow::LineOfSight(-normalized));
                    queue.push_back((new_dist, neigh_idx, source));
                }
            }
        }
    }
}

const NAV_LINE_OF_SIGHT_DIST: f32 = 30.;

pub fn generate_flow_field_impl(
    nav_grid: Arc<NavGridInner>,
    sources: Vec<[usize; 2]>,
) -> (Duration, FlowFieldInner) {
    let start = Instant::now();
    let mut flow_field = Array2::from_elem(nav_grid.grid.raw_dim(), (f32::INFINITY, Flow::None));

    // Do a first pass with normal BFS
    let mut queue = VecDeque::new();
    for source in sources.iter() {
        queue.push_back((0., *source));
        flow_field[*source] = (0., Flow::Source);
    }
    while let Some((dist, idx)) = queue.pop_front() {
        // Performance improvements (on level nav-stress-test, AMD 5800X3D):
        // - Check North, East, South, West before diagonals: 14x speedup !!!!
        // - Use a bitfield instead of checking all 8 directions: 1.5x speedup

        // Latest runtime is about 23ms, for some reason it jumps around though

        let grid_val = nav_grid.grid[idx];
        macro_rules! check_neighbor {
            ($flow:expr) => {
                check_neighbor($flow, dist, grid_val, idx, &mut flow_field, &mut queue)
            };
        }
        check_neighbor!(Flow::North);
        check_neighbor!(Flow::East);
        check_neighbor!(Flow::South);
        check_neighbor!(Flow::West);
        check_neighbor!(Flow::NorthEast);
        check_neighbor!(Flow::SouthEast);
        check_neighbor!(Flow::SouthWest);
        check_neighbor!(Flow::NorthWest);
    }

    // Do a second pass with line of sight raycasting
    let mut los_queue = VecDeque::new();
    for source in sources.iter() {
        los_queue.push_back((0., *source, *source));
    }
    while let Some((dist, idx, source)) = los_queue.pop_front() {
        if dist > NAV_LINE_OF_SIGHT_DIST {
            continue;
        }
        let grid_val = nav_grid.grid[idx];
        macro_rules! check_neighbor_raycast {
            ($flow:expr) => {
                check_neighbor_raycast(
                    $flow,
                    idx,
                    grid_val,
                    source,
                    &nav_grid,
                    &mut flow_field,
                    &mut los_queue,
                )
            };
        }
        check_neighbor_raycast!(Flow::North);
        check_neighbor_raycast!(Flow::East);
        check_neighbor_raycast!(Flow::South);
        check_neighbor_raycast!(Flow::West);
        check_neighbor_raycast!(Flow::NorthEast);
        check_neighbor_raycast!(Flow::SouthEast);
        check_neighbor_raycast!(Flow::SouthWest);
        check_neighbor_raycast!(Flow::NorthWest);
    }

    let elapsed = start.elapsed();

    (elapsed, flow_field)
}

fn is_point_in_polygon(point: Vec2, vertices: &Vertices) -> bool {
    if vertices.len() < 3 {
        return false;
    }
    // This algo is from copilot, don't ask me
    let mut odd_nodes = false;
    for (vj, vi) in iter::once(vertices.last().unwrap())
        .chain(vertices.iter())
        .zip(vertices.iter())
    {
        if ((vi.y < point.y && vj.y >= point.y) || (vj.y < point.y && vi.y >= point.y))
            && ((point.y - vi.y) / (vj.y - vi.y)).mul_add(vj.x - vi.x, vi.x) < point.x
        {
            odd_nodes = !odd_nodes;
        }
    }
    odd_nodes
}

fn is_clockwise(vertices: &Vertices) -> bool {
    let mut sum = 0.;
    for i in 0..vertices.len() {
        let vi = vertices[i];
        let vj = vertices[(i + 1) % vertices.len()];
        sum += (vj.x - vi.x) * (vj.y + vi.y);
    }
    sum > 0.
}

fn inflate_polygon(vertices: &Vertices, amount: f32) -> Option<Vertices> {
    if vertices.len() < 3 {
        return None;
    }

    let mut coords = vertices
        .iter()
        .map(|v| Coordinate {
            x: f64::from(v.x),
            y: f64::from(v.y),
        })
        .cycle()
        .take(vertices.len() + 1)
        .collect_vec();

    if is_clockwise(vertices) {
        coords.reverse();
    }

    // TODO: Detect failures properly (common failure is to return a very small or empty polygon)
    let lines = match offset_polygon(&coords.into(), f64::from(amount), 10.) {
        Ok(lines) => lines.first()?.clone(),
        Err(_) => {
            return None;
        }
    };

    lines
        .points_iter()
        .map(|c| Vec2::new(c.x() as f32, c.y() as f32))
        .collect_vec()
        .into()
}

fn generate_flow_field_system(world: &mut World) {
    let mut system_state: SystemState<(
        Res<NavGrid>,
        ResMut<FlowField>,
        Query<&Transform, With<Target>>,
        ResMut<Statistics>,
    )> = SystemState::new(world);
    let (nav_grid, mut flow_field, target_q, mut stats) = system_state.get_mut(world);

    // When the last player dies, just continue going towards the latest corpse
    let targets = target_q
        .iter()
        .map(|tr| find_valid_source(&nav_grid, tr.translation.truncate()))
        .collect::<Vec<_>>();

    let (duration, flow_field_inner) = generate_flow_field_impl(Arc::clone(&nav_grid), targets);
    stats.add("flow_field", duration);
    flow_field.0 = flow_field_inner;
}

#[derive(Resource, Default)]
struct FlowFieldGenerate {
    task: Option<Task<(Duration, FlowFieldInner)>>,
    last_started: Duration,
}
const MIN_NAV_GEN_INTERVAL: Duration = Duration::from_millis(200);
fn start_flow_field_generation_task(
    nav_grid: Res<NavGrid>,
    mut gen: ResMut<FlowFieldGenerate>,
    target_q: Query<&Transform, With<Target>>,
    time: Res<Time<Virtual>>,
    mut stats: ResMut<Statistics>,
) {
    let start = Instant::now();
    if gen.task.is_some() || time.elapsed() - gen.last_started < MIN_NAV_GEN_INTERVAL {
        stats.add("flow_field", start.elapsed());
        return;
    }
    // When the last player dies, just continue going towards the latest corpse
    let targets = target_q
        .iter()
        .map(|tr| find_valid_source(&nav_grid, tr.translation.truncate()))
        .collect::<Vec<_>>();

    let nav_grid = Arc::clone(&nav_grid);

    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move { generate_flow_field_impl(nav_grid, targets) });

    gen.task = Some(task);
    gen.last_started = time.elapsed();
    stats.add("flow_field", start.elapsed());
}

fn handle_flow_field_task(
    mut gen: ResMut<FlowFieldGenerate>,
    mut flow_field: ResMut<FlowField>,
    mut stats: ResMut<Statistics>,
) {
    let start = Instant::now();
    if let Some(task) = gen.task.as_mut() {
        if let Some((_, flow_field_res)) = future::block_on(future::poll_once(task)) {
            flow_field.0 = flow_field_res;
            gen.task = None;
        }
    }

    *stats.last_mut("flow_field").unwrap() += start.elapsed();
}

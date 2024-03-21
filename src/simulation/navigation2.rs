use crate::{
    level::{Level, LevelStartupSet},
    statistics::Statistics,
    utils::{ToUsizeArr, ToVec2, Vertices},
};
use bevy::{
    ecs::system::SystemState,
    prelude::*,
    utils::{HashSet, Instant},
};
use geo_types::Coordinate;
use itertools::Itertools;
use ndarray::Array2;
use std::{collections::VecDeque, f32::consts::SQRT_2, iter, sync::Arc, time::Duration};

use super::{spawning::ENEMY_RADIUS, SimulationSet};

pub const NAV_SCALE: f32 = ENEMY_RADIUS;
pub const NAV_SCALE_INV: f32 = 1. / NAV_SCALE;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlowField>()
            .init_resource::<IntegrationField>()
            .init_resource::<MousePos>()
            .add_systems(PreStartup, init_nav_grid.after(LevelStartupSet::Spawn))
            .add_systems(
                PreUpdate,
                (
                    generate_flow_field_system.in_set(SimulationSet::GenNavigation),
                    update_mouse_pos,
                ),
            );
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct NavGrid(pub Arc<NavGridInner>);

#[derive(Default)]
pub struct NavGridInner {
    #[allow(dead_code)]
    size: f32,
    inflated_walls: Vec<Vertices>,
    pub walkable: Array2<bool>,
    /// Contains bitsets of directions that can be moved in from a given index
    pub grid: Array2<u8>,
}

fn init_nav_grid(mut commands: Commands, level: Res<Level>) {
    let nav_grid = NavGridInner::new(level.size, &level.walls);
    commands.insert_resource(NavGrid(Arc::new(nav_grid)));
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

use offset_polygon::offset_polygon;

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

pub type FlowFieldInner = Array2<Flow>;

#[derive(Resource, Default)]
pub struct FlowField {
    pub targets: Vec<Vec2>,
    pub field: FlowFieldInner,
}

pub type IntegrationFieldInner = Array2<(u16, u8)>;

#[derive(Resource, Default)]
pub struct IntegrationField(pub IntegrationFieldInner);

impl FlowField {
    pub fn get(&self, idx: [usize; 2]) -> Option<&Flow> {
        self.field.get(idx)
    }
}

impl FlowField {
    pub fn closest_target(&self, pos: Vec2) -> Option<Vec2> {
        let mut closest = None;
        let mut closest_dist = f32::MAX;
        for &target in &self.targets {
            let dist = (target - pos).length_squared();
            if dist < closest_dist {
                closest = Some(target);
                closest_dist = dist;
            }
        }
        closest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Flow {
    #[default]
    None,
    North,
    East,
    South,
    West,
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
    LineOfSight,
}

impl Flow {
    pub fn to_dir(self) -> Vec2 {
        match self {
            Self::None => Vec2::ZERO,
            Self::North => Vec2::Y,
            Self::NorthEast => Vec2::new(SQRT_2 / 2., SQRT_2 / 2.),
            Self::East => Vec2::X,
            Self::SouthEast => Vec2::new(SQRT_2 / 2., -SQRT_2 / 2.),
            Self::South => Vec2::NEG_Y,
            Self::SouthWest => Vec2::new(-SQRT_2 / 2., -SQRT_2 / 2.),
            Self::West => Vec2::NEG_X,
            Self::NorthWest => Vec2::new(-SQRT_2 / 2., SQRT_2 / 2.),
            Self::LineOfSight => Vec2::ZERO,
        }
    }

    pub const fn distance(self) -> f32 {
        match self {
            Self::None | Self::LineOfSight => 0.,
            Self::North | Self::East | Self::South | Self::West => 1.,
            Self::NorthEast | Self::SouthEast | Self::SouthWest | Self::NorthWest => SQRT_2,
        }
    }

    #[inline]
    pub const fn mask(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::LineOfSight => 0,
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

/// Actually returns the "opposite" of the flow, this is used to find the neighbor
#[inline]
pub const fn neighbor_idx([x, y]: [usize; 2], flow: Flow) -> [usize; 2] {
    match flow {
        Flow::None | Flow::LineOfSight => {
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

const LINE_OF_SIGHT: u8 = 0b1;
const ACTIVE_WAVE_FRONT: u8 = 0b01;

#[inline(always)]
fn check_los(
    dir: Flow,
    val: u16,
    idx: [usize; 2],
    source: [usize; 2],
    nav_grid: &NavGridInner,
    integration_field: &mut Array2<(u16, u8)>,
    queue: &mut VecDeque<(u16, [usize; 2], [usize; 2])>,
    active_wave_fronts: &mut VecDeque<[usize; 2]>,
) {
    let neigh_idx = neighbor_idx(idx, dir);

    let Some(&walkable) = nav_grid.walkable.get(neigh_idx) else {
        return;
    };

    if !walkable {
        let (value, flags) = &mut integration_field[neigh_idx];
        if *flags & (LINE_OF_SIGHT | ACTIVE_WAVE_FRONT) != 0 {
            return;
        }

        let directions = [Flow::North, Flow::East, Flow::South, Flow::West];

        for (i, dir) in directions.iter().enumerate() {
            let outer_neigh_idx = neighbor_idx(neigh_idx, *dir);
            if nav_grid.walkable.get(outer_neigh_idx).is_some_and(|w| *w) {
                integration_field[outer_neigh_idx] = (val + 1, ACTIVE_WAVE_FRONT);
            };
        }

        return;
    }

    let (value, flags) = &mut integration_field[neigh_idx];
    if *flags & (LINE_OF_SIGHT | ACTIVE_WAVE_FRONT) != 0 {
        return;
    }

    *value = val + 1;
    *flags = LINE_OF_SIGHT;

    queue.push_back((val + 1, neigh_idx, source));
}

fn generate_flow_field_impl(
    nav_grid: Arc<NavGridInner>,
    sources: &[[usize; 2]],
) -> (Duration, FlowFieldInner, IntegrationFieldInner) {
    let start = Instant::now();

    // For second stage
    let mut active_wave_fronts: VecDeque<[usize; 2]> = VecDeque::new();
    let flow_field = Array2::from_elem(nav_grid.grid.raw_dim(), Flow::None);

    let mut integration_field = Array2::from_elem(nav_grid.grid.raw_dim(), (0u16, 0u8));
    let mut queue = VecDeque::new();

    for source in sources.iter() {
        queue.push_back((0u16, *source, *source));
        integration_field[*source] = (0, LINE_OF_SIGHT);
    }

    // Line of sight pass
    while let Some((val, idx, source)) = queue.pop_front() {
        macro_rules! check_los {
            ($flow:expr) => {
                check_los(
                    $flow,
                    val,
                    idx,
                    source,
                    &nav_grid,
                    &mut integration_field,
                    &mut queue,
                    &mut active_wave_fronts,
                )
            };
        }
        check_los!(Flow::North);
        check_los!(Flow::East);
        check_los!(Flow::South);
        check_los!(Flow::West);
    }

    (start.elapsed(), flow_field, integration_field)
}

pub fn find_valid_source(nav_grid: &NavGrid, pos: Vec2) -> [usize; 2] {
    let idx = nav_grid.pos_to_index(pos);
    if nav_grid.walkable().get(idx).is_some_and(|w| *w) {
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
    [1, 1]
}

fn generate_flow_field_system(world: &mut World) {
    let mut system_state: SystemState<(
        Res<NavGrid>,
        ResMut<FlowField>,
        // Query<&Transform, With<Target>>,
        Res<MousePos>,
        ResMut<Statistics>,
    )> = SystemState::new(world);
    let (nav_grid, mut flow_field, target_q, mut stats) = system_state.get_mut(world);

    // When the last player dies, just continue going towards the latest corpse
    // let targets = target_q
    //     .iter()
    //     .map(|tr| super::navigation::find_valid_source(&nav_grid, tr.translation.truncate()))
    //     .collect::<Vec<_>>();
    //
    let targets = vec![find_valid_source(&nav_grid, target_q.0)];

    let (duration, flow_field_inner, integration_field_inner) =
        generate_flow_field_impl(Arc::clone(&nav_grid), &targets);
    stats.add("flow_field", duration);

    *flow_field = FlowField {
        targets: targets
            .into_iter()
            .map(NavGridInner::index_to_pos)
            .collect(),
        field: flow_field_inner,
    };

    world.insert_resource(IntegrationField(integration_field_inner));
}

#[derive(Resource, Clone, Default)]
struct MousePos(Vec2);

fn update_mouse_pos(
    window: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut mouse_pos: ResMut<MousePos>,
) {
    let Ok(window) = window.get_single() else {
        return;
    };

    let get_mouse_pos = || -> Option<MousePos> {
        let (camera, camera_g_transform) = camera.single();

        let pos = window.cursor_position()?;
        Some(MousePos(
            camera.viewport_to_world_2d(camera_g_transform, pos)?,
        ))
    };

    if let Some(p) = get_mouse_pos() {
        *mouse_pos = p;
    }
}

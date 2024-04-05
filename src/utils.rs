use std::iter;

use bevy::prelude::*;

pub type Vertices = Vec<Vec2>;

pub trait ToVec2 {
    fn to_vec2(&self) -> Vec2;
}

impl ToVec2 for [usize; 2] {
    fn to_vec2(&self) -> Vec2 {
        Vec2::new(self[0] as f32, self[1] as f32)
    }
}

pub trait ToUsizeArr {
    fn to_usize_arr(&self) -> [usize; 2];
}

impl ToUsizeArr for Vec2 {
    fn to_usize_arr(&self) -> [usize; 2] {
        [self.x as usize, self.y as usize]
    }
}

pub trait ToAngle {
    fn to_angle(&self) -> f32;
}

impl ToAngle for Vec2 {
    fn to_angle(&self) -> f32 {
        self.y.atan2(self.x)
    }
}

pub fn square(size: f32) -> Vertices {
    rectangle(Vec2::splat(size))
}

pub fn rectangle(size: Vec2) -> Vertices {
    let half_width = size.x / 2.;
    let half_height = size.y / 2.;
    vec![
        Vec2::new(-half_width, -half_height),
        Vec2::new(half_width, -half_height),
        Vec2::new(half_width, half_height),
        Vec2::new(-half_width, half_height),
    ]
}

pub fn spatial(pos: Vec2, z: f32) -> SpatialBundle {
    SpatialBundle {
        transform: Transform::from_translation(pos.extend(z)),
        ..default()
    }
}

pub trait WithOffset {
    fn with_offset(self, offset: Vec2) -> Self;
}

impl WithOffset for Vertices {
    fn with_offset(self, offset: Vec2) -> Self {
        self.into_iter().map(|v| v + offset).collect()
    }
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Velocity(pub Vec2);

pub trait Easing {
    fn lerp(self, b: Self, f: f32) -> Self;
}

impl Easing for Color {
    fn lerp(self, b: Self, f: f32) -> Self {
        Color::rgba(
            self.r().lerp(b.r(), f),
            self.g().lerp(b.g(), f),
            self.b().lerp(b.b(), f),
            self.a().lerp(b.a(), f),
        )
    }
}

pub fn is_point_in_polygon(point: Vec2, vertices: &Vertices) -> bool {
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

pub fn is_clockwise(vertices: &Vertices) -> bool {
    let mut sum = 0.;
    for i in 0..vertices.len() {
        let vi = vertices[i];
        let vj = vertices[(i + 1) % vertices.len()];
        sum += (vj.x - vi.x) * (vj.y + vi.y);
    }
    sum > 0.
}

use geo_types::Coordinate;
use itertools::Itertools;
use offset_polygon::offset_polygon;

pub fn inflate_polygon(vertices: &Vertices, amount: f32) -> Option<Vertices> {
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

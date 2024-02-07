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

impl Easing for f32 {
    fn lerp(self, b: Self, f: f32) -> Self {
        self * (1. - f) + b * f
    }
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

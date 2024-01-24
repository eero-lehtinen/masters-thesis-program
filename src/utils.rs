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

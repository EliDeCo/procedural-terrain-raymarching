use bevy::{prelude::*};
//use bevy::math::DVec3;
use std::collections::HashSet;


#[derive(Component)]
pub struct Player{
    pub facing: Vec3
}

#[derive(Resource, Clone)]
pub struct PlanetMaterial(pub Handle<StandardMaterial>);

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ChunkKey {
    pub direction: IVec3,
    pub coords: IVec2,
    pub lod: u8,
}

#[derive(Resource, Default)]
pub struct RenderedChunks {
    pub set: HashSet<ChunkKey>,
}

#[derive(Component)]
pub struct Chunk(pub ChunkKey);



#[derive(Resource, Default)]
pub struct PlayerPos {
    pub position: Vec3,
}


/* 
pub trait ToDVec3 {
    fn to_dvec3(self) -> DVec3;
}

impl ToDVec3 for Vec3 {
    fn to_dvec3(self) -> DVec3 {
        DVec3 { x: (self.x as f64), y: (self.y as f64), z: (self.z as f64) }
    }
}

impl ToDVec3 for IVec3 {
    fn to_dvec3(self) -> DVec3 {
        DVec3 { x: (self.x as f64), y: (self.y as f64), z: (self.z as f64) }
    }
}

pub trait ToVec3 {
    fn to_vec3(self) -> Vec3;
}

impl ToVec3 for DVec3 {
    fn to_vec3(self) -> Vec3 {
        Vec3 { x: (self.x as f32), y: (self.y as f32), z: (self.z as f32) }
    }
}z
*/
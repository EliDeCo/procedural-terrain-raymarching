use bevy::prelude::*;
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
}

#[derive(Resource, Default)]
pub struct RenderedChunks {
    pub set: HashSet<ChunkKey>,
}

#[derive(Component)]
pub struct Chunk(pub ChunkKey);
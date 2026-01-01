use bevy::prelude::*;
use bevy::math::DVec3;
use std::collections::HashSet;

///used for identifying the player entity
#[derive(Component)]
pub struct Player {
    pub facing: Vec3,
}

///default material for terrain
#[derive(Resource, Clone)]
pub struct PlanetMaterial(pub Handle<StandardMaterial>);

///stores critical information about a chunk
#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ChunkKey {
    pub direction: IVec3,
    pub coords: IVec2,
    pub lod: u8,
}

///stores the set of currently rendered chunks
#[derive(Resource, Default)]
pub struct RenderedChunks {
    pub set: HashSet<ChunkKey>,
}

///used to connect chunk entities to their ChunkKey
#[derive(Component)]
pub struct Chunk(pub ChunkKey);

///stores the simulated position of the player, as well as the offset value used to convert it into rendered position
#[derive(Resource, Default)]
pub struct PlayerInfo {
    pub position: DVec3,
    pub offset: Vec3,
    pub facing: Vec3,
}
//use std::f32::consts::{SQRT_2};
use bevy::{
    prelude::*,
    render::{mesh::VertexAttributeValues}
};
//use bevy_rich_text3d::{Text3d, Text3dStyling, TextAtlas};
use std::collections::HashSet;

//use crate::data_structures::*;
use crate::constructs::*;


pub const PLANET_RADIUS: f32 = 100.; // in meters
const PREFERRED_CHUNK_SIZE: f32 = 10.; // in meters
const PREFERRED_SUBDIVISION_SIZE: f32 = 10.; // in meters



const SQRT_3: f32 = 1.7320508075688772; // sqrt(3) for convenience
const CUBE_SIZE: f32 = 2.*PLANET_RADIUS / SQRT_3; // side length of the cube that will become the planet
const HALF: f32 = CUBE_SIZE / 2.0; // half the size of the cube
const CHUNKS_PER_EDGE: u32 = (CUBE_SIZE / PREFERRED_CHUNK_SIZE) as u32; // number of chunks along one edge of a cube face
const ACTUAL_CHUNK_SIZE: f32 = CUBE_SIZE / CHUNKS_PER_EDGE as f32; // actual size of each chunk
const CHUNK_SUBDIVISIONS: u32 = (ACTUAL_CHUNK_SIZE / PREFERRED_SUBDIVISION_SIZE) as u32 - 1; // number of subdivisions in each chunk

pub fn display_info() {  
    info!("CUBE_SIZE: {}", CUBE_SIZE);
    info!("CHUNKS_PER_EDGE: {}", CHUNKS_PER_EDGE);
    info!("ACTUAL_CHUNK_SIZE: {}", ACTUAL_CHUNK_SIZE);
    info!("CHUNK_SUBDIVISIONS: {}", CHUNK_SUBDIVISIONS);
    info!("ACTUAL SUBDIVISION SIZE: {}", ACTUAL_CHUNK_SIZE/(CHUNK_SUBDIVISIONS as f32+1.));
}


fn generate_chunk_mesh(direction: IVec3, coords: IVec2, subdivisions: u32) -> Mesh {
    let mut mesh = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(ACTUAL_CHUNK_SIZE, ACTUAL_CHUNK_SIZE)
            .subdivisions(subdivisions),
    );

    //get the axis relative to the current face
    let (direction, rel_x, rel_y) = face_axes(direction);
    // get the rotation to align the chunk with the given face direction
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);

    // use the coordinates to find the offset for the chunk
    let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
    let x_offset = (coords.x as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;
    let y_offset = (coords.y as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;
    

    let transform = Transform {
        translation: direction*(HALF) + rel_x*x_offset + rel_y*y_offset,
        rotation: rotation,
        ..default()
    };

    // bake the cube transform into the mesh vertices
    bake_rigid_transform(&mut mesh, transform);

    // back the spherical transform
    bake_spherical_transform(&mut mesh);

    mesh.duplicate_vertices();
    mesh.compute_flat_normals();

    mesh
}

fn bake_rigid_transform(mesh: &mut Mesh, transform: Transform) {
    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for pos in positions {
            *pos = transform.transform_point(Vec3::from_array(*pos)).to_array();
        }
    }
}

// Convert the cube vertices to a spherical surface
fn bake_spherical_transform(mesh: &mut Mesh) {
    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for pos in positions {

            // takes the vector pointing from the center of the cube and extends it to the planet radius
            *pos = (Vec3::from_array(*pos).normalize() * PLANET_RADIUS).to_array();
           
        }
    }
}



//takes in the current player location in 3D and returns a Chunkkey direction and cooresponding chunk coords
fn get_chunk_key(coords: Vec3) -> ChunkKey {

    let a = coords.abs();
    let largest = a.x.max(a.y).max(a.z);
    let direction = if largest == coords.x {
        IVec3::X
    } else if largest == -coords.x {
        IVec3::NEG_X
    } else if largest == coords.y {
        IVec3::Y
    } else if largest == -coords.y {
       IVec3::NEG_Y
    } else if largest == coords.z {
        IVec3::Z
    } else if largest == -coords.z {
        IVec3::NEG_Z
    } else {
        panic!("What!?!?")
    };


    //get relative coordinate data
    let (direction, rel_x, rel_y) = face_axes(direction);

    //Get 3D location where the line that intersects the player passes through a chunk (when it was still on the flat cube face)
    let parallel_component = coords.dot(direction);
    let distance = HALF / parallel_component;
    let face_projection = coords * distance;


    //opposite of calculations used to find 3D coordinates from chunkcoords
    let x_offset = face_projection.dot(rel_x);
    let y_offset = face_projection.dot(rel_y);


    let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
    let chunk_coords = IVec2::new(
        ((x_offset / ACTUAL_CHUNK_SIZE) + half - 0.5).round() as i32,
        ((y_offset / ACTUAL_CHUNK_SIZE) + half - 0.5).round() as i32,
    );

    let direction = to_ivec3(direction);

    ChunkKey {
        direction: direction,
        coords: chunk_coords
    }
}



//Handles drawing and deleting chunks as necessary
pub fn manage_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    //mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
    mut rendered: ResMut<RenderedChunks>,
    planet_material: Res<PlanetMaterial>,
    all_chunks: Query<(Entity, &Chunk)>,
) {
    //USE INSERT
    if let Ok(player_transform) = player_q.single() {
        //let (direction, chunk_coords) = get_chunk_coords(player_transform.translation);

        //println!("Chunk COORDS: {}", chunk_coords)

        //get the list of chunks to be rendered
        let to_render = assign_chunks(player_transform.translation);

        //remove chunks that are now out of range or do not have the correct LOD(not included in to_render)
        for (entity, chunk) in all_chunks {
            if !to_render.contains(&chunk.0) {
                commands.entity(entity).despawn();
                rendered.set.remove(&chunk.0);
            }
        }

        //if the chunk is not already rendered (or needs to be rerendered with a different LOD), render it
        for chunk in to_render {
            if rendered.set.insert(chunk.clone()) {
                let for_handle = chunk.clone();
                let handle = meshes.add(generate_chunk_mesh(for_handle.direction, for_handle.coords, CHUNK_SUBDIVISIONS));
                commands.spawn((
                    Mesh3d(handle),
                    MeshMaterial3d(planet_material.0.clone()),
                    Chunk(for_handle),
                ));

                

            }
        }
    }
}

//assigns visible chunks to be rendered
fn assign_chunks(player_coords: Vec3) -> HashSet<ChunkKey> {

    let mut to_render: HashSet<ChunkKey> = HashSet::new();

    let player_chunk = get_chunk_key(player_coords);

    let render: i32 = 5;


    for x in -render..=render {
        for y in -render..=render {
            if x*x + y*y <= render*render {
                to_render.insert(
                player_to_global(&player_chunk, ivec2(x, y))
                );
            }
            
        }
    }


    to_render
}

//converts from chunk coordinates centered around the player (treating the sphere surface as a flat plane) to a ChunkKey
fn player_to_global(player: &ChunkKey, relative_coords: IVec2) -> ChunkKey {

    //get the axes relative to the current face
    //These will be the axes used to interpret relative_coords
    let (_, rel_x, rel_y) = face_axes(player.direction);

    //this point will be moved along the sphere by the loops following the instructions given by relative_coords
    let mut point: ChunkKey = *player;

    //these are the directions we will be travelling in (they may change if we wrap sides)
    let mut x_dir: Vec3 = rel_x;
    let mut y_dir: Vec3 = rel_y;
    let mut nx_dir: Vec3 = -rel_x;
    let mut ny_dir: Vec3 = -rel_y;

    //move in the given directions
    for _ in 0..relative_coords.x {
        move_in_direction(&mut point, &mut x_dir);
    }
    for _ in 0..(-relative_coords.x) {
        move_in_direction(&mut point, &mut nx_dir);
    }
    for _ in 0..relative_coords.y {
        move_in_direction(&mut point, &mut y_dir);
    }
    for _ in 0..(-relative_coords.y) {
        move_in_direction(&mut point, &mut ny_dir);
    }


    point
}

//moves a Chunkkey in a direction in 3D space, wrapping the chunk and the move direction as necessary
fn move_in_direction(chunk: &mut ChunkKey, dir: &mut Vec3) {
    //get the 3d position of the chunk on the surface of the cube and move it by 1 chunk in the given direction
    let pos = chunk_center_on_cube(*chunk) + (*dir * ACTUAL_CHUNK_SIZE);
    //if the new position is still within the bounds of the face, simply return new position
    let (face, rel_x, rel_y) = face_axes(chunk.direction);
    if pos.dot(rel_x) <= HALF - (ACTUAL_CHUNK_SIZE / 2.) && pos.dot(rel_y) <= HALF - (ACTUAL_CHUNK_SIZE / 2.) && pos.dot(-rel_x) <= HALF - (ACTUAL_CHUNK_SIZE / 2.) && pos.dot(-rel_y) <= HALF - (ACTUAL_CHUNK_SIZE / 2.) { //to lazy to write a script determining which direction I actually need to use
        *chunk = get_chunk_key(pos.normalize() * PLANET_RADIUS);
    } else {
        //if the new position is not within the bounds of the face, wrap the position to the next side and update the movement direction
        let wrapped_pos = pos - (*dir * (ACTUAL_CHUNK_SIZE/2.)) - (face * (ACTUAL_CHUNK_SIZE/2.));
        *chunk = get_chunk_key(wrapped_pos.normalize() * PLANET_RADIUS);
        *dir = -face;
    }
}

//calculate the local axis of a given face
fn face_axes(direction: IVec3) -> (Vec3, Vec3, Vec3) {
    // Face normal as unit Vec3
    let dir = Vec3::new(direction.x as f32, direction.y as f32, direction.z as f32).normalize();
    // Rotate +Y (plane normal) onto face normal
    let rot = Quat::from_rotation_arc(Vec3::Y, dir);
    // Tangent axes on that face
    let rel_x = (rot * Vec3::X).normalize();
    let rel_y = (rot * Vec3::Z).normalize();
    (dir, rel_x, rel_y)
}

//convert Vec3 to Ivec3
fn to_ivec3(input: Vec3) -> IVec3 {IVec3::new(input.x as i32, input.y as i32, input.z as i32)}

//convert Ivec3 to Vec3
//fn to_vec3(input: IVec3) -> Vec3 {Vec3::new(input.x as f32, input.y as f32, input.z as f32)}


//given a chunk, it finds the 3D coordinates of its center on the surface of the cube
fn chunk_center_on_cube(chunk: ChunkKey) -> Vec3 {
    let (dir, rel_x, rel_y) = face_axes(chunk.direction);

    let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
    let x_offset = (chunk.coords.x as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;
    let y_offset = (chunk.coords.y as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;

    (dir * HALF) + (rel_x * x_offset) + (rel_y * y_offset)
}

/*
//shows the coordinates of all chunks on the planet
pub fn show_coords(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    //mut meshes: ResMut<Assets<Mesh>>,
) {
    let text_mat = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        //base_color: Color::srgb(0.5, 0., 0.),
        ..Default::default()
    });


    for direction in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
        for x in 0..(CHUNKS_PER_EDGE as i32) {
             for y in 0..(CHUNKS_PER_EDGE as i32) {
                let chunk = ChunkKey {
                    direction: direction,
                    coords: ivec2(x, y),
                };

                //get the axis relative to the current face
                let (direction, rel_x, rel_y) = face_axes(direction);
                // get the rotation to align the chunk with the given face direction
                let rotation = Quat::from_rotation_arc(Vec3::Z, direction);

                // use the coordinates to find the offset for the chunk
                let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
                let x_offset = (chunk.coords.x as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;
                let y_offset = (chunk.coords.y as f32 - half + 0.5) * ACTUAL_CHUNK_SIZE;


                let transform = Transform {
                    translation: (direction*(HALF+0.1) + rel_x*x_offset + rel_y*y_offset).normalize() * PLANET_RADIUS,
                    rotation: rotation,
                    ..default()
                };

                let for_handle = chunk.clone();
                //let handle = meshes.add(generate_chunk_mesh(for_handle.direction, for_handle.coords, CHUNK_SUBDIVISIONS));
                commands.spawn((
                    Text3d::new(format!("({},{})", for_handle.coords.x, for_handle.coords.y)),
                    Mesh3d::default(),
                    MeshMaterial3d(text_mat.clone()),
                    transform,
                    Visibility::Visible,
                    Text3dStyling {
                        size: 60.,
                        color: Srgba { red: (1.), green: (0.), blue: (0.), alpha: (1.) },
                        ..default()
                    }
                ));
            }
        }
    }
}
     */
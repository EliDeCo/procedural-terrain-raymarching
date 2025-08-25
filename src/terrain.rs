//use std::f32::consts::{SQRT_2};
use bevy::{
    prelude::*,
    render::mesh::VertexAttributeValues,
};

//use crate::data_structures::*;
use crate::constructs::*;

pub const PLANET_RADIUS: f32 = 20.; // in meters
const PREFERRED_CHUNK_SIZE: f32 = 4.; // in meters
const PREFERRED_SUBDIVISION_SIZE: f32 = 4.; // in meters



const SQRT_3: f32 = 1.7320508075688772; // sqrt(3) for convenience
const CUBE_SIZE: f32 = 2.*PLANET_RADIUS / SQRT_3; // side length of the cube that will become the planet
const HALF: f32 = CUBE_SIZE / 2.0; // half the size of the cube
const CHUNKS_PER_EDGE: u32 = (CUBE_SIZE / PREFERRED_CHUNK_SIZE) as u32; // number of chunks along one edge of a cube face
const ACTUAL_CHUNK_SIZE: f32 = CUBE_SIZE / CHUNKS_PER_EDGE as f32; // actual size of each chunk
const CHUNK_SUBDIVISIONS: u32 = (ACTUAL_CHUNK_SIZE / PREFERRED_SUBDIVISION_SIZE) as u32 - 1; // number of subdivisions in each chunk

pub fn generate_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    //player_q: Query<&Transform, With<Player>>,
) {

// spawn basic cube
    //commands.spawn((
    //    Mesh3d(meshes.add(Cuboid::new(5., 5., 5.))),
    //    MeshMaterial3d(materials.add(Color::srgb(0.8, 0.5, 0.3)))
    //));

    // create planet material
    let planet_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.5, 0.3),
        perceptual_roughness: 0.5,
        metallic: 0.5,
        ..default()
    });

    
    info!("CUBE_SIZE: {}", CUBE_SIZE);
    info!("CHUNKS_PER_EDGE: {}", CHUNKS_PER_EDGE);
    info!("ACTUAL_CHUNK_SIZE: {}", ACTUAL_CHUNK_SIZE);
    info!("CHUNK_SUBDIVISIONS: {}", CHUNK_SUBDIVISIONS);
    info!("ACTUAL SUBDIVISION SIZE: {}", ACTUAL_CHUNK_SIZE/(CHUNK_SUBDIVISIONS as f32+1.));
    
    
    for dir in [Vec3::Y, Vec3::X, Vec3::Z,Vec3::NEG_Y, Vec3::NEG_X, Vec3::NEG_Z] {
        for x in 0..CHUNKS_PER_EDGE {
            for y in 0..CHUNKS_PER_EDGE {
                //if y == 2 { continue;}
                let mesh = generate_chunk_mesh(dir, Vec2::new(x as f32, y as f32), CHUNK_SUBDIVISIONS);
                // spawn the chunk mesh with the material
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(planet_material.clone()),
                    //transform,
                ));
            }
        }
    }
    
    /* 
    if let Ok(player_transform) = player_q.single() {
        let (dir, coords) = get_chunk_coords(player_transform.translation);
        println!("DIRECTION: {}", dir);
        let mesh = generate_chunk_mesh(dir, Vec2::new(2., 2.),CHUNK_SUBDIVISIONS);
        commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(planet_material.clone()),
        ));
    }
    */

    



}


fn generate_chunk_mesh(direction: Vec3, coords: Vec2, subdivisions: u32) -> Mesh {
    let mut mesh = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(ACTUAL_CHUNK_SIZE, ACTUAL_CHUNK_SIZE)
            .subdivisions(subdivisions),
    );
    // get the rotation to align the chunk with the given face direction
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
    // get the relative x and y (horizontal and vertical) axes on the given chunk face
    let rel_x = (rotation * Vec3::X).normalize();
    let rel_y = (rotation * Vec3::Z).normalize();

    // use the coordinates to find the offset for the chunk
    let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
    let x_offset = (coords.x - half + 0.5) * ACTUAL_CHUNK_SIZE;
    let y_offset = (coords.y - half + 0.5) * ACTUAL_CHUNK_SIZE;
    

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





//---------For inverse mapping
//Dormalize player coords (we just need the direction, not the distance)
//Find the largest value of the player coordinates. For example, y is the largest and it's positive, then the player is on the top face of the cube
//Find the local coordinates by scaling the player direction until it reaches the edge of the cube face
//find chunk coordinates by undoing offset calculations and rounding (the inverse of let x_offset = (coords.x - half  + 0.5) * ACTUAL_CHUNK_SIZE;))
//calculate horizon angle and use it to determine which chunks are visible

//takes in the current player location in 3D and returns the direction and cooresponding chunk coords
fn get_chunk_coords(coords: Vec3) -> (Vec3, Vec2) {

    let a = coords.abs();
    let largest = a.x.max(a.y).max(a.z);
    let direction = if largest == coords.x {
        Vec3::X
    } else if largest == -coords.x {
        Vec3::NEG_X
    } else if largest == coords.y {
        Vec3::Y
    } else if largest == -coords.y {
        Vec3::NEG_Y
    } else if largest == coords.z {
        Vec3::Z
    } else if largest == -coords.z {
        Vec3::NEG_Z
    } else {
        panic!("What!?!?")
    };

    //let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
    //let rel_x = (rotation * Vec3::X).normalize();
    //let rel_y = (rotation * Vec3::Z).normalize();

    //Get 3D location where the line that intersects the player passes through a chunk (when it was still on the flat cube face)
    let parallel_component = coords.dot(direction);
    let distance = (HALF) / parallel_component;
    let face_projection = coords * distance;

    //get relative coordinate data
    let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
    let rel_x = (rotation * Vec3::X).normalize();
    let rel_y = (rotation * Vec3::Z).normalize();


    //opposite of calculations used to find 3D coordinates from chunkcoords
    let x_offset = face_projection.dot(rel_x);
    let y_offset = face_projection.dot(rel_y);


    let half: f32 = CHUNKS_PER_EDGE as f32 / 2.0;
    let chunk_coords = Vec2::new(
        ((x_offset / ACTUAL_CHUNK_SIZE) + half - 0.5).round(),
        ((y_offset / ACTUAL_CHUNK_SIZE) + half - 0.5).round(),
    );

    //let y_offset = (coords.y - half + 0.5) * ACTUAL_CHUNK_SIZE;

    //println!("FACE PROJECTION: {}", face_projection);

    return (direction, chunk_coords);
}

//TODO: Make global material
pub fn manage_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_q.single() {
        let (direction, chunk_coords) = get_chunk_coords(player_transform.translation);

        //println!("Chunk COORDS: {}", chunk_coords)

    let planet_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.5, 0.3),
        perceptual_roughness: 0.5,
        metallic: 0.5,
        ..default()
    });

    let mesh = generate_chunk_mesh(direction, chunk_coords, CHUNK_SUBDIVISIONS);
    //let mesh = generate_chunk_mesh(direction, Vec2::new(-1., -1.), CHUNK_SUBDIVISIONS);

    // spawn the chunk mesh with the material
    commands.spawn((
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(planet_material.clone()),
    ));

    }


}
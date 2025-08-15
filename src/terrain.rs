use std::f32::consts::SQRT_2;
use bevy::{
    prelude::*,
    render::mesh::VertexAttributeValues,
};

//use crate::data_structures::*;


const PLANET_RADIUS: f32 = 8.0; // in meters
const PREFERRED_CHUNK_SIZE: f32 = 3.; // in meters on the flat cube face
const CHUNK_SUBDIVISIONS: u32 = 4; // number of subdivisions per chunk


const CUBE_SIZE: f32 = PLANET_RADIUS * SQRT_2; // side length of the cube that will become the planet
const CHUNKS_PER_FACE: u32 = (CUBE_SIZE / PREFERRED_CHUNK_SIZE) as u32; // number of chunks along one edge of a cube face
const ACTUAL_CHUNK_SIZE: f32 = CUBE_SIZE / CHUNKS_PER_FACE as f32; // actual size of each chunk

pub fn generate_planet(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
    info!("CHUNKS_PER_FACE: {}", CHUNKS_PER_FACE);
    info!("ACTUAL_CHUNK_SIZE: {}", ACTUAL_CHUNK_SIZE);


    for dir in [Vec3::Y, Vec3::X, Vec3::Z,Vec3::NEG_Y, Vec3::NEG_X, Vec3::NEG_Z] {
        for x in 0..CHUNKS_PER_FACE {
            for y in 0..CHUNKS_PER_FACE {
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
    let half: f32 = CHUNKS_PER_FACE as f32 / 2.0;
    let x_offset = (coords.x - half  + 0.5) * ACTUAL_CHUNK_SIZE;
    let y_offset = (coords.y - half + 0.5) * ACTUAL_CHUNK_SIZE;

    let transform = Transform {
        translation: direction*(CUBE_SIZE/2.) + rel_x*x_offset + rel_y*y_offset,
        rotation: rotation,
        ..default()
    };

    // bake the cube transform into the mesh vertices
    bake_rigid_transform(&mut mesh, transform);

    mesh
}

fn bake_rigid_transform(mesh: &mut Mesh, transform: Transform) {
    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for pos in positions {
            *pos = transform.transform_point(Vec3::from_array(*pos)).to_array();
        }
    }
}
/* 
fn bake_spherical_transform(mesh: &mut Mesh, rel_x: Vec3, rel_y: Vec3, rel_z: Vec3) {
    if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
        for pos in positions {

        }
    }
}
    */
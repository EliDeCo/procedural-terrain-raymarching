use bevy::{ 
    pbr::wireframe::Wireframe, 
    prelude::*, render::mesh::VertexAttributeValues, 
    //render::{render_resource::WgpuFeatures, settings::{RenderCreation, WgpuSettings}, RenderPlugin,}
};
use noise::{NoiseFn, Perlin};
use rand::{self, Rng};


pub fn spawn_heightmap(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let seed: u32 = rand::rng().random(); // seed for Perlin noise
    //let terrain_height = 20.; // height of terrain in Plane3d units
    let subdivision_width = 50.; // size of subdivision in Plane3d units
    let terrain_width = 10000.; // size of terrain in Plane3d units
    let noise = Perlin::new(seed); // create a new Perlin noise generator with the given seed

    // create a Plane3d mesh with the specified size and subdivisions
    let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(terrain_width, terrain_width)
            .subdivisions((terrain_width / subdivision_width).round() as u32)
    );
    
    fn apply_noise(pos: &mut [f32;3], noise: &Perlin, amplitude: f32, period: f32) -> () {
        let val = noise.get([
            (pos[0] / period) as f64, 
            (pos[2] / period) as f64,
            ]);
        pos[1] += val as f32 * amplitude;
    }


    if let Some(VertexAttributeValues::Float32x3(
        positions,
    )) = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        // iterate over the positions and apply Perlin noise to the y-coordinate
        for pos in positions.iter_mut() {
            //texture
            apply_noise(pos, &noise, 20., 300.);

            //hills
            apply_noise(pos, &noise, 120., 1000.);
        }
        // TODO: fix color system depending on noise octaves later
        // determine color based on the y-coordinate (height)
        let colors: Vec<[f32; 4]> = positions
            .iter()
            .map(|[x, _, z]| {
                //let y = *y / terrain_height * 2.;
                //let y = *y/2.0 / terrain_height;

                //if y > 0.85 {
                    // white for snow
                //    Color::srgba(5., 5., 5., 1.).to_linear().to_f32_array()
                //} else if y > 0.75 {
                    // gray for rock
                //    Color::srgba(0.5, 0.5, 0.5, 1.).to_linear().to_f32_array()
                //} else if y > 0.35{
                    // green for grass
                
                let val = (noise.get([
                    *x as f64 / 10.,
                    *z as f64 / 10.,
                ]) / 10.) + 1.;


                (Color::srgba(0.3, 0.5, 0.2, 1.).to_linear()
                * val as f32)
                .to_f32_array()

                //} else {
                    //yellow for sand
                //    Color::srgba(0.8, 0.7, 0.4, 1.).to_linear().to_f32_array()
                
            })
            .collect();
        terrain.insert_attribute(
            Mesh::ATTRIBUTE_COLOR, 
            colors,
        );
    }
    // calculate normals for flat shading
    terrain.duplicate_vertices();
    terrain.compute_flat_normals();

    // create a Mesh3d component with the terrain mesh
    commands.spawn((
        Mesh3d(meshes.add( terrain)),
        MeshMaterial3d(materials.add(StandardMaterial {
            //base_color: Color::srgb(0.1, 0.4, 0.1),
            perceptual_roughness: 0.9,
            reflectance: 0.02,
            ..default()
        })),
        Terrain,
    ));
}

#[derive(Component)]
pub struct Terrain;

// toggles visibility of wireframe on meshes with Terrain component
pub fn toggle_wireframe(
    mut commands: Commands,
    landscapes_wireframe: Query<
        Entity,
        (With<Terrain>, With<Wireframe>),
    >,
    landscapes: Query<
        Entity,
        (With<Terrain>, Without<Wireframe>),
    >,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::KeyP) {
        for terrain in &landscapes {
            commands.entity(terrain).insert(Wireframe);
        }
        for terrain in &landscapes_wireframe {
            commands.entity(terrain).remove::<Wireframe>();
        }
    }
}
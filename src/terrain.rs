use bevy::{ 
    pbr::wireframe::Wireframe, 
    platform::collections::HashMap, prelude::*, 
    render::mesh::VertexAttributeValues,
    //render::{render_resource::WgpuFeatures, settings::{RenderCreation, WgpuSettings}, RenderPlugin,}
};
use noise::{NoiseFn, Perlin};
//use rand::{self, Rng};

//This is called from main.rs to generate terrain every tick
pub fn spawn_heightmap(
    mut commands: Commands,
    //mut meshes: ResMut<Assets<Mesh>>,
    //mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //spawn the first 9 chunks
    for x in -20..=20 {
        for y in -20..=20 {
            commands.queue(SpawnTerrain(IVec2::new(x, y)));
        }
    }
}

// apply two instances of noise rotate by 90 degrees with the given properties
fn apply_noise(
    pos: &mut [f32;3], 
    noise: &Perlin, 
    amplitude: f32, 
    period: f32, 
    power: f32,
    chunk_coords: IVec2,
    mesh_size: f32,
    ) {
    
    // adjust position relative to world
    let world_x = pos[0] + (mesh_size * chunk_coords.x as f32);
    let world_z = pos[2] + (mesh_size * chunk_coords.y as f32);

    // apply noise
    let val1 = noise.get([world_x as f64 / period as f64, world_z as f64 / period as f64]);
    let val2 = noise.get([world_z as f64 / period as f64, world_x as f64 / period as f64]);

    pos[1] += (val1 + val2).powf(power as f64) as f32 * amplitude;
}


#[derive(Resource)]
pub struct TerrainStore(pub HashMap<IVec2, Handle<Mesh>>);

struct SpawnTerrain(IVec2);

impl Command for SpawnTerrain {
    fn apply(self, world: &mut World) {
        if world
            .get_resource_mut::<TerrainStore>()
            .expect("TerrainStore to be available")
            .0
            .get(&self.0)
            .is_some()
        {
            // mesh already exists
            // do nothing for now
            warn!("mesh {} already exists", self.0);
            return;
        };
        let noise = Perlin::new(2007); // create a new Perlin noise generator with the given seed

        let mesh_size = 256.;

        //1 unit = 1 meter
        // each chunk is 256x256 meters with 31 subdivisions (32 vertices per side)
        let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(mesh_size, mesh_size)
            .subdivisions(31)
        );

        if let Some(VertexAttributeValues::Float32x3(
            positions,
        )) =
        terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // iterate over the positions and apply Perlin noise to the y-coordinate
            for pos in positions.iter_mut() {
                //base texture
                apply_noise(
                    pos, 
                    &noise, 
                    0.5, 
                    30.,
                    1.0,
                    self.0,
                    mesh_size,
                );

                //rolling hills
                apply_noise(
                    pos, 
                    &noise, 
                    10., 
                    500.,
                    2.0,
                    self.0,
                    mesh_size,
                );

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

        // add the mesh to the world
        let mesh = world
            .get_resource_mut::<Assets<Mesh>>()
            .expect("meshes db to be available")
            .add(terrain);
        let material = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .expect("StandardMaterial db to be available")
            .add(Color::WHITE);

        world
            .get_resource_mut::<TerrainStore>()
            .expect("TerrainStore to be available")
            .0
            .insert(self.0, mesh.clone());

        world.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_xyz(
                self.0.x as f32 * mesh_size,
                0.,
                self.0.y as f32 * mesh_size,
            ),
            Terrain,
        ));
    }
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
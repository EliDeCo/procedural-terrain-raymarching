use bevy::{ 
    pbr::wireframe::Wireframe, 
    prelude::*, render::mesh::VertexAttributeValues, 
    //render::{render_resource::WgpuFeatures, settings::{RenderCreation, WgpuSettings}, RenderPlugin,}
};
use noise::{NoiseFn, Perlin};


pub fn spawn_heightmap(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let seed: u32 = 17;

    let terrain_height = 40.;

    let mut terrain = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(10000., 10000.)
            .subdivisions(100)
    );
    
    if let Some(VertexAttributeValues::Float32x3(
        positions,
    )) = terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        let perlin = Perlin::new(seed);

        for pos in positions.iter_mut() {
            let val = perlin.get([
                pos[0] as f64 / 300., 
                pos[2] as f64 / 300.,
                ]);
            pos[1] = val as f32 * terrain_height;
        }
    }
    terrain.duplicate_vertices();
    terrain.compute_flat_normals();

    commands.spawn((
        Mesh3d(meshes.add( terrain)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.4, 0.1),
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
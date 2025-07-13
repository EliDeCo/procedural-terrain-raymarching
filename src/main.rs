use std::f32::consts::FRAC_PI_4;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    platform::collections::HashMap,
    prelude::*,
    render::{
        RenderPlugin,
        mesh::VertexAttributeValues,
        render_resource::WgpuFeatures,
        settings::{RenderCreation, WgpuSettings},
    },
    window::{CursorGrabMode, PresentMode, PrimaryWindow},
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;
use noise::{NoiseFn, Perlin};

const CHUNK_SIZE: f32 = 512.; // size of each chunk in meters
const CHUNK_SUBDIVISIONS: u32 = 63; // subdivisions per chunk (N+1 vertices per side)
const RENDER_DISTANCE: i32 = 9; // how many chunks to render around the player
const PLAYER_SPEED: f32 = 0.1; // speed of the player ship
const SEED: u32 = 2007; // seed for the Perlin noise generator


fn main() {
    App::new()
        .insert_resource(TerrainStore(HashMap::default()))
        .add_plugins((
            DefaultPlugins
                //for disabling fps cap (vsync)
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                })
                //for wireframe rendering
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    }),
                    ..default()
                }),
            WireframePlugin::default(),
            PanOrbitCameraPlugin,
        ))
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })
        //sky color
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_systems(Startup, (setup, spawn_heightmap))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                control_ship,
                sync_camera_to_ship,
                manage_chunks,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 200.0, 0.0)),
        PanOrbitCamera::default(),
        ShipCam,
    ));

    //Character stand-in
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 2., 1.))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
        Ship,
    ));

    // lock mouse into window by default
    if let Ok(mut primary_window) = q_windows.single_mut() {
        primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
        primary_window.cursor_options.visible = false;
        let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
        primary_window.set_cursor_position(Some(center));
    }

    //add sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -FRAC_PI_4, -FRAC_PI_4, 0.0)),
    ));

    // performance UI
    commands.spawn((
        PerfUiEntryFPS::default(),
        PerfUiEntryFPSAverage::default(),
        PerfUiEntryFrameTime::default(),
        PerfUiEntryRenderCpuTime::default(),
        PerfUiEntryRenderGpuTime::default(),
        PerfUiEntryEntityCount::default(),
    ));
}

fn grab_mouse(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut primary_window) = q_windows.single_mut() {
        if mouse.just_pressed(MouseButton::Left) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
            primary_window.cursor_options.visible = false;
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));
        }

        if key.just_pressed(KeyCode::Escape) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::None;
            primary_window.cursor_options.visible = true;
        }
    }
}

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct ShipCam;

fn control_ship(input: Res<ButtonInput<KeyCode>>, mut ships: Query<&mut Transform, With<Ship>>) {
    let mut direction = Vec2::new(0., 0.);
    if input.pressed(KeyCode::KeyW) {
        direction.y += PLAYER_SPEED;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= PLAYER_SPEED;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x += PLAYER_SPEED;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x -= PLAYER_SPEED;
    }
    for mut ship in &mut ships {
        ship.translation.x += direction.x;
        ship.translation.z += direction.y;
    }
}

fn sync_camera_to_ship(
    mut pan_orbit_q: Query<&mut PanOrbitCamera>,
    cube_q: Query<&Transform, With<Ship>>,
) {
    if let Ok(mut pan_orbit) = pan_orbit_q.single_mut() {
        if let Ok(cube_tfm) = cube_q.single() {
            pan_orbit.target_focus = cube_tfm.translation;
            pan_orbit.force_update = true;
        }
    }
}

fn spawn_heightmap(
    mut commands: Commands,
    //mut meshes: ResMut<Assets<Mesh>>,
    //mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //spawn the first few chunks

    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for y in -RENDER_DISTANCE..=RENDER_DISTANCE {
            let offset = IVec2::new(x, y);
            if offset.length_squared() <= RENDER_DISTANCE * RENDER_DISTANCE {
                commands.queue(SpawnTerrain(offset));
            }
        }
    }
}

// apply two instances of noise rotate by 90 degrees with the given properties
fn apply_noise(
    pos: &mut [f32; 3],
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
    let val1 = noise.get([
        world_x as f64 / period as f64,
        world_z as f64 / period as f64,
    ]);
    let val2 = noise.get([
        world_z as f64 / period as f64,
        world_x as f64 / period as f64,
    ]);

    pos[1] += (val1 + val2).powf(power as f64) as f32 * amplitude;
}

#[derive(Resource)]
struct TerrainStore(pub HashMap<IVec2, Handle<Mesh>>);

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
        let noise = Perlin::new(SEED); // create a new Perlin noise generator with the given seed

        //1 unit = 1 meter
        // each chunk is 256x256 meters with 31 subdivisions (32 vertices per side)
        let mut terrain = Mesh::from(
            Plane3d::default()
                .mesh()
                .size(CHUNK_SIZE, CHUNK_SIZE)
                .subdivisions(CHUNK_SUBDIVISIONS),
        );

        if let Some(VertexAttributeValues::Float32x3(positions)) =
            terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            // iterate over the positions and apply Perlin noise to the y-coordinate
            for pos in positions.iter_mut() {
                //base texture
                apply_noise(pos, &noise, 0.5, 30., 1.0, self.0, CHUNK_SIZE);

                //rolling hills
                apply_noise(pos, &noise, 10., 500., 2.0, self.0, CHUNK_SIZE);
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

                    let val = (noise.get([*x as f64 / 10., *z as f64 / 10.]) / 10.) + 1.;

                    (Color::srgba(0.3, 0.5, 0.2, 1.).to_linear() * val as f32).to_f32_array()

                    //} else {
                    //yellow for sand
                    //    Color::srgba(0.8, 0.7, 0.4, 1.).to_linear().to_f32_array()
                })
                .collect();
            terrain.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
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
            Transform::from_translation(Vec3::new(
                (self.0.x as f32 * CHUNK_SIZE).round(),
                0.0,
                (self.0.y as f32 * CHUNK_SIZE).round(),
            )),
            Terrain,
        ));
    }
}

#[derive(Component)]
struct Terrain;

fn toggle_wireframe(mut config: ResMut<WireframeConfig>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        config.global = !config.global;
    }
}


fn manage_chunks(
    mut commands: Commands,
    mut current_chunk: Local<IVec2>,
    ship: Query<&Transform, With<Ship>>,
    mut terrain_store: ResMut<TerrainStore>,
    terrain_entities: Query<
        (Entity, &Mesh3d),
        With<Terrain>,
    >,
) {

    let Ok(transform) = ship.single() else {
        warn!("no ship!");
        return;
    };

    let xz = (transform.translation.xz() / CHUNK_SIZE)
        .trunc()
        .as_ivec2();

    if *current_chunk != xz {
        *current_chunk = xz;

        let mut chunks_to_render = Vec::new();

        for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let offset = IVec2::new(dx, dz);
                if offset.length_squared() <= RENDER_DISTANCE * RENDER_DISTANCE {
                    chunks_to_render.push(*current_chunk + offset);
                }
            }
        }
        // extract_if is perfect here, but its nightly
        let chunks_to_despawn: Vec<(IVec2, Handle<Mesh>)> =
            terrain_store
                .0
                .clone()
                .into_iter()
                .filter(|(key, _)| {
                    !chunks_to_render.contains(&key)
                })
                .collect();

        for (chunk, mesh) in chunks_to_despawn {
            let Some((entity, _)) = terrain_entities
                .iter()
                .find(|(_, mesh3d)| mesh3d.0 == mesh)
            else {
                continue;
            };
            commands.entity(entity).despawn();
            terrain_store.0.remove(&chunk);
        }

        for chunk in chunks_to_render {
            commands.queue(SpawnTerrain(chunk));
        }
    }
}


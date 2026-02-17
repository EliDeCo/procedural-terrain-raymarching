use std::{collections::HashMap, f32::EPSILON};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowResolution},
};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;

const VOXEL_SIZE_INPUT: u32 = 2;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;
const FRAC_PI_4: f32 = std::f32::consts::FRAC_PI_4;
const MOVE_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = FRAC_PI_4;
const VOXEL_SIZE: f32 = VOXEL_SIZE_INPUT as f32;
const INV_VOXEL_SIZE: f32 = 1.0 / VOXEL_SIZE;
const EPS: f32 = 1e-5;
const RENDER_DISTANCE: f32 = 50.;
const RENDER_DIST_VOXELS: i32 = (RENDER_DISTANCE / VOXEL_SIZE) as i32;
const BUFFER_SIZE: i32 = RENDER_DIST_VOXELS as i32 * 2;

//TODO: Impliment "sphere" tracing but subtract 1 voxel length to avoid overshoot
//TODO: Fix vertical rays (again)

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate,
                    title: "Bevy".into(),
                    resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_plugins(WireframePlugin { ..default() })
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .init_resource::<TerrainStore>()
        .add_systems(Startup, (setup, /*init_terrain.after(setup)*/))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                update_cam,
                player_move,
                update_fps_text,
                update_terrain,
            ),
        )
        .add_systems(PostUpdate, march_rays.after(TransformSystems::Propagate))
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((Camera3d::default(),));

    // lock mouse into window by default
    if let Ok((mut primary_window, mut cursor_options)) = q_window.single_mut() {
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
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

    //Player mockup
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 2., 1.))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.3))),
        Transform::from_xyz(0.0, 1.0, 0.0),
        Player {
            forward: Vec3::NEG_Z,
            up: Vec3::Y,
            right: Vec3::X,
        },
    ));

    //setup global material handle
    let planet_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.49, 0.22),
        metallic: 0.0,
        perceptual_roughness: 0.90,
        reflectance: 0.04,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });

    commands.insert_resource(PlanetMaterial(planet_material.clone()));

    //terrain base
    /*
    commands.spawn((
        Mesh3d(
            meshes.add(
                Plane3d::default()
                    .mesh()
                    .size(20.0, 20.)
                    .subdivisions(9)
                    .build(),
            ),
        ),
        MeshMaterial3d(planet_material),
    ));
    */

    //fps text
    commands
        .spawn((Text::new("Fps:"), TextColor(Color::BLACK)))
        .with_child((TextSpan::default(), TextColor(Color::BLACK), FpsText));

    //noise for terrain generation
    let seed = 9226;
    commands.insert_resource(NoiseStore {
        basic_perlin: Perlin::new(seed),
    });

    //TODO: MAKE THIS UPDATE
    commands.insert_resource(MaxHeight(8.));
}

//Takes the x and z voxel coordinates of a quad and returns the index within the terrain buffer
fn get_index(x: i32, z: i32) -> usize {
    positive_mod(z, BUFFER_SIZE) * BUFFER_SIZE as usize + positive_mod(x, BUFFER_SIZE)
}
/* 
fn init_terrain(noise: Res<NoiseStore>, mut terrain_store: ResMut<TerrainStore>) {
    //spawn terrain
    for x in -RENDER_DIST_VOXELS..RENDER_DIST_VOXELS {
        for z in RENDER_DIST_VOXELS..RENDER_DIST_VOXELS {
            let idx = get_index(x, z);
            terrain_store.quad_buffer[idx] = QuadInfo::default();
            //to_add.push((coords, QuadInfo::new_simple(coords, &noise)));           
        }
    }

    /* 
    let mut to_add = Vec::new();
    for x in -32..32 {
        for y in -32..32 {
            let coords = IVec2::new(x, y);
            to_add.push((coords, QuadInfo::new_simple(coords, &noise)));
        }
    }

    terrain_store.quad_map.extend(to_add);
    */
}
    */

fn grab_mouse(
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if let Ok((mut primary_window, mut cursor_options)) = q_window.single_mut() {
        if mouse.just_pressed(MouseButton::Left) {
            cursor_options.grab_mode = CursorGrabMode::Locked;
            cursor_options.visible = false;
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));
        }

        if key.just_pressed(KeyCode::Escape) {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

fn toggle_wireframe(key: Res<ButtonInput<KeyCode>>, mut config: ResMut<WireframeConfig>) {
    if key.just_pressed(KeyCode::Space) {
        config.global = !config.global;
    }
}

fn update_cam(
    camera_q: Single<&mut Transform, With<Camera>>,
    player_q: Single<&Transform, (With<Player>, Without<Camera>)>,
    acc_mouse_motion: Res<AccumulatedMouseMotion>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    cursor_q: Single<&CursorOptions, With<PrimaryWindow>>,
) {
    let mut camera_transform = camera_q.into_inner();
    let cursor_options = *cursor_q;
    let player_transform = *player_q;

    camera_transform.translation = player_transform.translation + Vec3::new(0.0, 1.75, 0.0);

    if cursor_options.grab_mode != CursorGrabMode::Locked {
        mouse.release_all();
        return;
    }

    let delta = acc_mouse_motion.delta;
    if delta != Vec2::ZERO {
        let yaw_rotation = Quat::from_rotation_y(-delta.x * 0.003);
        let pitch_rotation = Quat::from_rotation_x(-delta.y * 0.002);

        camera_transform.rotation *= yaw_rotation;

        let possible_pitch = ((camera_transform.rotation * pitch_rotation) * Vec3::NEG_Z)
            .y
            .asin();
        if possible_pitch.abs() < PITCH_LIMIT {
            camera_transform.rotation *= pitch_rotation;
        }
        let forward: Vec3 = camera_transform.forward().into();
        //keep roll at zero
        camera_transform.look_to(forward, Vec3::Y);
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &mut Transform)>,
) {
    if let Ok((player, mut player_transform)) = player_q.single_mut() {
        let mut input = Vec3::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyE) {
            input.z += 1.0;
        }
        if keys.pressed(KeyCode::KeyQ) {
            input.z -= 1.0;
        }
        if input != Vec3::ZERO {
            input = input.normalize();
        }

        let move_direction =
            (player.right * input.x + player.forward * input.y + player.up * input.z).normalize();

        if input.length_squared() > 1e-6 {
            player_transform.translation += move_direction * MOVE_SPEED * time.delta_secs();
        }
    }
}

/// Updates the FPS text display.
fn update_fps_text(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut TextSpan, With<FpsText>>,
    player_q: Query<&Transform, With<Player>>,
) {
    if let Ok(mut span) = query.single_mut() {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
            && let Some(value) = fps.smoothed()
        {
            let pos = if let Ok(player_transform) = player_q.single() {
                player_transform.translation
            } else {
                Vec3::ZERO
            };
            let coords = coord(pos);
            **span = format!("{value:.2}  {coords}");
        }
    }
}

//get the height of a given point in world coornates
fn get_height(x: i32, z: i32, noise: &NoiseStore) -> f32 {
    noise.basic_perlin.get([x as f64 + FRAC_PI_4 as f64, z as f64 + FRAC_PI_4 as f64]) as f32
}

///Raymarches in 2D space along the xz plane, testing when the heightmap is collided with
fn march_rays(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window>,
    mut gizmos: Gizmos,
    terrain_store: Res<TerrainStore>,
    max_height: Res<MaxHeight>,
    noise: Res<NoiseStore>,
) {
    let (camera, camera_transform) = *camera_query;

    //for testing

    let width = window.width();
    let height = window.height();
    let row1 = height * 0.6;
    let row2 = height * 0.8;
    let columns: Vec<f32> = (1..8).map(|x| width * (x as f32 * 0.125)).collect();

    let pixels: Vec<Vec2> = [row1, row2]
        .iter()
        .flat_map(|row| {
            columns
                .iter()
                .map(|col| Vec2::new(col.to_owned(), row.to_owned()))
        })
        .collect();


    //let pixels = [Vec2::new(width / 2., row2)];

    for pixel in pixels {
        let ray = camera.viewport_to_world(camera_transform, pixel).unwrap();
        traverse(&ray, max_height.0, &terrain_store, &mut gizmos);
    }

    //temporary "rendering"
    //for (_, quad) in &terrain_store.quad_map {
    for quad in &terrain_store.quad_buffer {
        let coords = IVec2::new(quad.world_coords.x as i32, quad.world_coords.y as i32);
        let v0 = coords;
        let v1 = coords + IVec2::new(VOXEL_SIZE as i32, 0);
        let v2 = coords + IVec2::new(0, VOXEL_SIZE as i32);
        let v3 = coords + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32);

        for edge in [
            (v0, v1),
            (v1, v2),
            (v2, v0),
            /*(v1, v2),*/ (v2, v3),
            (v3, v1),
        ] {
            let start = Vec3::new(
                edge.0.x as f32,
                get_height(edge.0.x, edge.0.y, &noise),
                edge.0.y as f32,
            );
            let end = Vec3::new(
                edge.1.x as f32,
                get_height(edge.1.x, edge.1.y, &noise),
                edge.1.y as f32,
            );
            gizmos.line(start, end, Color::BLACK);
        }
    }

    //fastest method
    /* 
    (0..window.width() as i32).into_par_iter().for_each(|x| {
        for y in 0..window.height() as i32 {
            let pixel = Vec2::new(x as f32, y as f32);
            let ray = camera.viewport_to_world(camera_transform, pixel).unwrap();
            traverse(&ray, max_height.0, &mut terrain_store);
        }
    });
    */
}

fn traverse(
    ray: &Ray3d,
    max_height: f32,
    terrain_store: &TerrainStore, 
    gizmos: &mut Gizmos,
) {
    let mut current_voxel: IVec2 = coord(ray.origin);
    let ray_origin_y = ray.origin.y;
    let mut ray_end_y = ray_origin_y;
    let ray_dir_y = ray.direction.y;
    let ray_dir_xz = ray.direction.xz();
    let tilted_up = if ray_dir_y > 0.0 { true } else { false };

    //Near vertical ray
    if ray_dir_xz.length_squared() < EPSILON {
        //if y is postive, no collision will happen and skip entirely
        if ray_dir_y.is_sign_positive() {
            return;
        }
        /* 
        //if y is negative, simply find terrain height in the starting voxel and use that as our final value
        if let Some(voxel) = terrain_store.quad_map.get(&current_voxel) {
            if let Some(_) = voxel.check_lower(ray) {
                //gizmos.sphere(Isometry3d::from_translation(point), 0.25, Color::BLACK);
            } else if let Some(_) = voxel.check_upper(ray) {
                //gizmos.sphere(Isometry3d::from_translation(point), 0.25, Color::BLACK);
            } else {
                info!("We missed");
            }
        }
        */
        info!("You shouldn't be here");
        return;
    }

    //collision offest in voxel coordinates
    let offset = Vec2::new(
        if ray.direction.x > 0.0 { 1.0 } else { 0.0 },
        if ray.direction.z > 0.0 { 1.0 } else { 0.0 },
    );

    //the next boundary for collison testing in world coordinates
    let next_boundary = Vec2::new(
        (current_voxel.x as f32 + offset.x) * VOXEL_SIZE,
        (current_voxel.y as f32 + offset.y) * VOXEL_SIZE,
    );

    //must be initalized per component so we can insert INFINITY to avoid NaN
    let mut t = Vec2::ZERO;
    let mut delta = Vec2::ZERO;

    // X axis
    if ray_dir_xz.x != 0.0 {
        let inv = 1.0 / ray_dir_xz.x;
        delta.x = VOXEL_SIZE * inv.abs();
        t.x = (next_boundary.x - ray.origin.x) * inv;
    } else {
        delta.x = f32::INFINITY;
        t.x = f32::INFINITY;
    }

    // Z axis
    if ray_dir_xz.y != 0.0 {
        let inv = 1.0 / ray_dir_xz.y;
        delta.y = VOXEL_SIZE * inv.abs();
        t.y = (next_boundary.y - ray.origin.z) * inv;
    } else {
        delta.y = f32::INFINITY;
        t.y = f32::INFINITY;
    }

    //which way to step in voxel coordinates
    let step = IVec2::new(
        if ray.direction.x > 0.0 {
            1
        } else if ray.direction.x < 0.0 {
            -1
        } else {
            0
        },
        if ray.direction.z > 0.0 {
            1
        } else if ray.direction.z < 0.0 {
            -1
        } else {
            0
        },
    );

    let t_max = RENDER_DISTANCE / ray_dir_xz.length();
    let mut t_current = 0.0;
    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end_y > max_height {
            //let end_point = ray.origin + ray.direction * t_current;
            //gizmos.sphere(Isometry3d::from_translation(end_point), 0.05, Color::BLACK);
            break;
        }

        //collision detection
        let t_next = t.x.min(t.y);

        let enter_point = ray.origin + ray.direction * t_current;
        let exit_point = ray.origin + ray.direction * t_next;

        //current_voxel is of type IVec2. current_voxel.y corresponds to its z coordinate
        let idx = get_index(current_voxel.x, current_voxel.y);

        if let Some(point) = terrain_store.quad_buffer[idx].intersect(enter_point, exit_point, ray /*gizmos*/) {
            //ray has hit terrain
            //println!("Intersected with voxel {}, world ({})", INV_VOXEL_SIZE * voxel.world_coords, voxel.world_coords);
            gizmos.sphere(Isometry3d::from_translation(point), 0.25, Color::BLACK);
            break;
        }


        /* 
        if let Some(voxel) = terrain_store.quad_map.get(&current_voxel) {
            if let Some(_) = voxel.intersect(enter_point, exit_point, ray /*gizmos*/) {
                //ray has hit terrain
                //println!("Intersected with voxel {}, world ({})", INV_VOXEL_SIZE * voxel.world_coords, voxel.world_coords);
                //gizmos.sphere(Isometry3d::from_translation(point), 0.25, Color::BLACK);
                break;
            }
        } else {
            //we are outside of currently rendered chunks (aka past render distance)
            /*
            gizmos.sphere(
                Isometry3d::from_translation(enter_point),
                0.15,
                Color::WHITE,
            );
            */
            break;
        }
        */

        //Traversal
        // see which plane is intersected first

        if t.x < t.y {
            //intersected with x plane first
            current_voxel.x += step.x;
            t.x += delta.x;
        } else {
            //intersected with z plane first
            current_voxel.y += step.y;
            t.y += delta.y;
        }

        t_current = t_next;
        ray_end_y = ray_origin_y + t_current * ray_dir_y;

        if t_current > t_max {
            //reached render distance
            break;
        }
    }
}

///used for identifying the player entity
#[derive(Component)]
pub struct Player {
    pub forward: Vec3,
    pub up: Vec3,
    pub right: Vec3,
}

///default material for terrain
#[derive(Resource, Clone)]
pub struct PlanetMaterial(pub Handle<StandardMaterial>);

///marker component for fps text
#[derive(Component)]
struct FpsText;

///Struct for holding all currently relevant terrain data
#[derive(Resource)]
struct TerrainStore {
    quad_buffer: Box<[QuadInfo]>,
}

impl Default for TerrainStore {
    fn default() -> Self {
        TerrainStore { 
            quad_buffer: (0..(BUFFER_SIZE*BUFFER_SIZE))
                .map(|_|QuadInfo::default())
                .collect()
        }
    }
}

impl TerrainStore {
    /*
    ///Tests if any of the recently added quads have a higher max height than the current value,
    /// and updates accordingly
    fn add_test(added: &[QuadInfo], max_height: &mut MaxHeight) {
        let max_added = added.iter().map(|q| q.y_max).reduce(f32::max).unwrap_or(0.0);
        if max_added > max_height.0 {
            max_height.0 = max_added;
        }
    }

    ///Tests if the quad with the largest max height was removed, and
    /// updates accordingly
    fn remove_test(&self, removed: &[QuadInfo], max_height: &mut MaxHeight) {
        for quad in removed {
            if quad.y_max == max_height.0 {
                max_height.0 = self
                    .quad_map
                    .iter()
                    .map(|(_, q)| q.y_max)
                    .reduce(f32::min)
                    .unwrap_or(0.0);
                break;
            }
        }

    }
    */
}

///Plane struct optimized for ray-plane intersection
#[derive(Debug, Default, Clone)]
struct SimplePlane {
    n: Vec3, // normalized normal
    d: f32,  // plane constant
}

impl SimplePlane {
    ///Returns the 3d point where the ray intersects the plane, or None if near parallel
    fn ray_plane(&self, ray: &Ray3d) -> Option<Vec3> {
        // denominator = dot(n, ray direction)
        let denom = self.n.dot(ray.direction.into());

        // Parallel (or extremely close to parallel)
        if denom.abs() < EPS {
            return None;
        }

        let t = -(self.n.dot(ray.origin) + self.d) / denom;

        // Optional: reject intersections behind the ray
        if t < EPS {
            return None;
        }

        Some(ray.origin + ray.direction * t)
    }
}

///converts world position to horizontal voxel coordinates
fn coord(p: Vec3) -> IVec2 {
    (p.xz() * INV_VOXEL_SIZE).floor().as_ivec2()
}

///Stores all relevant information of a quad for ray intersection calculations
#[derive(Debug, Clone)]
struct QuadInfo {
    world_coords: Vec2, //coordinates of the vertex with the lowest (closest to negative infinity) x and z values in world space
    voxel_coords: IVec2,
    upper: SimplePlane,
    lower: SimplePlane,
    y_max: f32,
    y_min: f32,
}

impl Default for QuadInfo {
    fn default() -> Self {
        QuadInfo {
            world_coords: Vec2::default(),
            voxel_coords: IVec2::MAX,
            upper: SimplePlane::default(),
            lower: SimplePlane::default(),
            y_max: 0.0,
            y_min: 0.0,
        }
    }
}

impl QuadInfo {
    ///Creates a new quad at the gixen voxel coordinates
    fn new_simple(coords: IVec2, noise: &NoiseStore) -> Self {
        let coords = VOXEL_SIZE as i32 * coords;
        let v0 = coords.to_array();
        let v1 = (coords + IVec2::new(VOXEL_SIZE as i32, 0)).to_array();
        let v2 = (coords + IVec2::new(0, VOXEL_SIZE as i32)).to_array();
        let v3 = (coords + IVec2::new(VOXEL_SIZE as i32, VOXEL_SIZE as i32)).to_array();

        return QuadInfo::new([v0, v1, v2, v3], noise);
    }

    ///Creates a new quad from 4 vertexes (given as [[x,z]; 4])
    fn new(vertexes: [[i32; 2]; 4], noise: &NoiseStore) -> Self {
        // Find min and max X/Z
        let x_min = vertexes.iter().map(|v| v[0]).min().unwrap();
        let z_min = vertexes.iter().map(|v| v[1]).min().unwrap();
        let x_max = vertexes.iter().map(|v| v[0]).max().unwrap();
        let z_max = vertexes.iter().map(|v| v[1]).max().unwrap();

        let world_coords = Vec2::new(x_min as f32, z_min as f32);
        let voxel_coords = coord(Vec3::new(world_coords.x, 0.0, world_coords.y));

        // Heights at the 4 corners
        let y0 = get_height(x_min, z_min, noise);
        let y1 = get_height(x_max, z_min, noise);
        let y2 = get_height(x_min, z_max, noise);
        let y3 = get_height(x_max, z_max, noise);
        /*
        let tester = if world_coords == Vec2::new(0.0, 2.0) {
            true
        } else {
            false
        };

        if tester {
            println!("Vertexes:");
            for v in [[x_min as f32,y0,z_min as f32],[x_max as f32,y1,z_min as f32],[x_min as f32,y2,z_max as f32],[x_max as f32,y3,z_max as f32]] {
                println!("{:#?}", v);
            }
        }
        */
        // Compute min/max Y
        let y_max = y0.max(y1).max(y2).max(y3);
        let y_min = y0.min(y1).min(y2).min(y3);

        // Construct lower plane (right angle at min_x, min_z)
        let lower = {
            let nx = -(y1 - y0) / VOXEL_SIZE;
            let ny = 1.0;
            let nz = -(y2 - y0) / VOXEL_SIZE;
            let n = Vec3::new(nx, ny, nz).normalize();
            let world_point = Vec3::new(x_min as f32, y0, z_min as f32);
            let d = -n.dot(world_point);
            SimplePlane { n, d }
        };

        // Construct upper plane (right angle at max_x, max_z)
        let upper = {
            let nx = -(y3 - y2) / VOXEL_SIZE;
            let ny = 1.0;
            let nz = -(y3 - y1) / VOXEL_SIZE;
            let n = Vec3::new(nx, ny, nz).normalize();
            let world_point = Vec3::new(x_max as f32, y3, z_max as f32);
            let d = -n.dot(world_point);
            SimplePlane { n, d }
        };

        QuadInfo {
            world_coords,
            voxel_coords,
            upper,
            lower,
            y_max,
            y_min,
        }
    }

    /*
    Vertexes:
    v0 = [
        0.0,
        -0.35355338,
        2.0,
    ],
    v1 = [
        2.0,
        -0.35355338,
        2.0,
    ],
    v2 = [
        0.0,
        0.0,
        4.0,
    ],
    v3 = [
        2.0,
        0.70710677,
        4.0,
    ]

    Upper is built from v1, v2, and v3
    Lower is built from v0, v1, and v2,
    */

    ///Returns the point where a 3d ray intersects with terrain generated by a heightmap stored as quads (if it exists)
    fn intersect(
        &self,
        enter_point: Vec3,
        exit_point: Vec3,
        ray: &Ray3d,
        //gizmos: &mut Gizmos,
    ) -> Option<Vec3> {
        if (enter_point.y > self.y_max && exit_point.y > self.y_max)
            || (enter_point.y < self.y_min && exit_point.y < self.y_min)
        {
            return None; //fully above terrain or below terrain
        }
        /*
        let tester = if self.world_coords == Vec2::new(0.0, 2.0) {
            true
        } else {
            false
        };
        */

        //check lower first
        if enter_point.z < exit_point.z || enter_point.x < exit_point.x {
            if let Some(point) = self.check_lower(ray) {
                return Some(point);
            } else {
                return self.check_upper(ray);
            }
        //check upper first
        } else {
            if let Some(point) = self.check_upper(ray) {
                return Some(point);
            } else {
                return self.check_lower(ray);
            }
        }
    }

    fn check_lower(&self, ray: &Ray3d) -> Option<Vec3> {
        let point = self.lower.ray_plane(ray);

        //if the ray does intersect the plane, make sure it happens within the voxel
        if let Some(point) = point {
            if coord(point) != self.voxel_coords {
                return None;
            }

            //if within the voxel, further make sure its within the triangle
            let point_2d = Vec2::new(point.x, point.z) - self.world_coords; //convert to local coordinates
            if point_2d.x + point_2d.y <= VOXEL_SIZE {
                return Some(point);
            }
        }
        return None;
    }

    fn check_upper(&self, ray: &Ray3d) -> Option<Vec3> {
        let point = self.upper.ray_plane(ray);

        //if the ray does intersect the plane, make sure it happens within the voxel
        if let Some(point) = point {
            if coord(point) != self.voxel_coords {
                return None;
            }

            //if within the voxel, further make sure its within the triangle
            let point_2d = Vec2::new(point.x, point.z) - self.world_coords; //convert to local coordinates
            if point_2d.x + point_2d.y >= VOXEL_SIZE {
                return Some(point);
            }
        }
        return None;
    }
}

fn positive_mod(a: i32, b: i32) -> usize {
    (((a % b) + b) % b) as usize
}

///Updates stale quads witin render distance
fn update_terrain(
    player_q: Single<&Transform, With<Player>>,
    mut terrain_store: ResMut<TerrainStore>,
    noise: Res<NoiseStore>
) {
    let player_voxel = coord(player_q.into_inner().translation);
    
    // Load voxels within render distance
    for z in (player_voxel.y - RENDER_DIST_VOXELS)..(player_voxel.y + RENDER_DIST_VOXELS) {
        for x in (player_voxel.x - RENDER_DIST_VOXELS)..(player_voxel.x + RENDER_DIST_VOXELS) {
            let idx = get_index(x, z);
            let voxel_coords = IVec2::new(x, z);
            
            // Only generate if stale
            if terrain_store.quad_buffer[idx].voxel_coords != voxel_coords {
                terrain_store.quad_buffer[idx] = QuadInfo::new_simple(voxel_coords, &noise);
            }
        }
    }
}

#[derive(Resource)]
///Noise Resources for terrain generation
struct NoiseStore {
    basic_perlin: Perlin,
}

#[derive(Resource)]
///Stores the max height of current rendered terrain
struct MaxHeight(f32);


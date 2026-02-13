use core::f32;
use std::f32::EPSILON;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions, PresentMode, PrimaryWindow, WindowResolution},
    input::mouse::AccumulatedMouseMotion
};
use rayon::prelude::*;

const WINDOW_WIDTH: u32 = 960;
const WINDOW_HEIGHT: u32 = 540;
const FRAC_PI_4: f32 = std::f32::consts::FRAC_PI_4;
const MOVE_SPEED: f32 = 10.0;
const PITCH_LIMIT: f32 = FRAC_PI_4;
const VOXEL_SIZE: u32 = 1;
const INV_VOXEL_SIZE: f32 = 1.0 / VOXEL_SIZE as f32;

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
        .add_systems(Startup, (setup /* enable_auto_indirect.after(setup) */,))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                update_cam,
                player_move,
                update_fps_text,
                march_rays
            ),
        )
        //.add_systems(PostUpdate, march_rays.after(TransformSystems::Propagate))
        .run();
}

fn setup(
    mut commands: Commands,
    mut q_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
    ));

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
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(10.))),
        MeshMaterial3d(planet_material),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    //fps text
    commands
        .spawn((Text::new("Fps:"), TextColor(Color::BLACK)))
        .with_child((TextSpan::default(), TextColor(Color::BLACK), FpsText));
}

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

        let possible_pitch = ((camera_transform.rotation
            * pitch_rotation)
            * Vec3::NEG_Z)
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


///converts world position to horizontal voxel coordinates
fn coord(p: Vec3) -> IVec2 {
    (p.xz() * INV_VOXEL_SIZE).floor().as_ivec2()
}

//placeholder for a heightmap
fn get_height(x: i32, z: i32) -> f32 {
    let _ = x + z;
    0.0
}

///Raymarches in 2D space along the xz plane, testing when the heightmap is collided with
fn march_rays(
    camera_query: Single<(&Camera, &GlobalTransform)>, 
    window: Single<&Window>,
) {
    let render_distance = 5000.0;
    let max_height = 10.0;

    let (camera, camera_transform) = *camera_query;

    (0..window.width() as i32)
        .into_par_iter()
        .for_each(|x| {
            for y in 0..window.height() as i32 {
                let pixel = Vec2::new(x as f32, y as f32);
                let ray = camera.viewport_to_world(camera_transform, pixel).unwrap();
                traverse(&ray, render_distance, max_height);
            }
        });
}

//TODO: Add correct collision detection with triangle intersection and flyby optimization
fn traverse(ray: &Ray3d, render_distance: f32, max_height: f32) {
    let mut current_voxel: IVec2 = coord(ray.origin);
    let ray_origin_y = ray.origin.y;
    let mut ray_end_y = ray_origin_y;
    let ray_dir_y = ray.direction.y;
    let ray_dir_xz = ray.direction.xz();
    let tilted_up = if ray_dir_y > 0.0 { true } else { false };

    if ray_dir_xz.length_squared() < EPSILON {
        //near vertical ray
        //if y is negative, simply find terrain height in the starting voxel and use that as our final value
        //if y is postive, no collision will happen and skip entirely
        return; 
    }

    //collision offest in voxel coordinates
    let offset = Vec2::new(
        if ray.direction.x > 0.0 { 1.0 } else { 0.0 },
        if ray.direction.z < 0.0 { 1.0 } else { 0.0 },
    );

    //the next boundary for collison testing in world coordinates
    let next_boundary = Vec2::new(
    (current_voxel.x as f32 + offset.x) * VOXEL_SIZE as f32,
    (current_voxel.y as f32 + offset.y) * VOXEL_SIZE as f32,
    );


    //must be initalized per component so we can insert INFINITY to avoid NaN
    let mut t = Vec2::ZERO;
    let mut delta = Vec2::ZERO;

    // X axis
    if ray_dir_xz.x != 0.0 {
        let inv = 1.0 / ray_dir_xz.x;
        delta.x = VOXEL_SIZE as f32 * inv.abs();
        t.x = (next_boundary.x - ray.origin.x) * inv;
    } else {
        delta.x = f32::INFINITY;
        t.x = f32::INFINITY;
    }

    // Z axis
    if ray_dir_xz.y != 0.0 {
        let inv = 1.0 / ray_dir_xz.y;
        delta.y = VOXEL_SIZE as f32 * inv.abs();
        t.y = (next_boundary.y - ray.origin.z) * inv;
    } else {
        delta.y = f32::INFINITY;
        t.y = f32::INFINITY;
    }

    let t_max = render_distance / ray_dir_xz.length();

    //which way to step in voxel coordinates
    let step = IVec2::new(
        if ray.direction.x > 0.0 { 1 } 
        else if ray.direction.x < 0.0 { -1 } 
        else { 0 },

        if ray.direction.z < 0.0 { 1 }
        else if ray.direction.z > 0.0 { -1 }
        else { 0 },
    );
    
    
    loop {
        //If our ray is tilted up and is above the highest known terrain, we can stop marching as it will never collide
        if tilted_up && ray_end_y > max_height {
            break;
        }

        if ray_end_y <= get_height(current_voxel.x, current_voxel.y) {
            //we have hit the terrain, so we can stop marching
            break;
        }
        //Traversal
        // see which plane is intersected first
        let t_current: f32;
        if t.x < t.y {
            //intersected with x plane first
            t_current = t.x;
            current_voxel.x += step.x;
            t.x += delta.x;
            //ray_end = ray.origin + t.x * ray.direction;
            ray_end_y = ray_origin_y + t_current * ray_dir_y;

        } else {
            //intersected with z plane first
            t_current = t.y;
            current_voxel.y += step.y;
            t.y += delta.y;
            ray_end_y = ray_origin_y + t_current * ray_dir_y;
        }

        //stop after reaching render distnace
        if t_current > t_max {
            break;
        }

    if t.x.is_nan() || t.y.is_nan() {
        panic!("NaN in t: {:?}", t);
    }

    if !t_max.is_finite() {
        panic!("Bad t_max");
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

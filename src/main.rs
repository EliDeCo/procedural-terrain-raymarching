mod terrain;
mod constructs;

use std::f32::consts::FRAC_PI_4;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin}, prelude::*, render::{
        render_resource::WgpuFeatures, renderer::RenderAdapterInfo, settings::{Backends, RenderCreation, WgpuSettings}, view::NoIndirectDrawing, RenderPlugin
    }, window::{CursorGrabMode, PresentMode, PrimaryWindow, WindowResolution}
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iyes_perf_ui::prelude::*;

use constructs::*;

use crate::terrain::PLANET_RADIUS;

const WINDOW_SCALE: f32 = 0.6;
const MOVE_SPEED: f32 = 50.; // m/s


const WINDOW_WIDTH: f32 = 1920. * WINDOW_SCALE;
const WINDOW_HEIGHT: f32 = 1080. * WINDOW_SCALE;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                //for disabling fps cap (vsync)
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        title: "Planet Generator".into(),
                        resolution: WindowResolution::new(WINDOW_WIDTH, WINDOW_HEIGHT),
                        position: WindowPosition::Centered(MonitorSelection::Primary),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                //for wireframe rendering
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        #[cfg(any(target_os = "windows", target_os = "linux"))]
                        backends: Some(Backends::VULKAN | Backends::DX12),
                        //backends: Some(Backends::DX12),
                        ..default()
                    }),
                    ..default()
                }),
            PanOrbitCameraPlugin,
        ))
        //sky color
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        //wireframe
        .add_plugins(WireframePlugin {..default()})
        .insert_resource(WireframeConfig {
            global: false,
            ..default()
        })


        //diagnostics
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_systems(Startup, (
            //print_display_resolution,
            setup, 
            enable_auto_indirect.after(setup),
            terrain::generate_planet, 
        ))
        .add_systems(
            Update,
            (
                grab_mouse,
                toggle_wireframe,
                follow_cam,
                player_move,
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
        Transform::from_translation(Vec3::new(terrain::PLANET_RADIUS*2., terrain::PLANET_RADIUS*2., terrain::PLANET_RADIUS*2.)),
        PanOrbitCamera::default(),
        //NoIndirectDrawing,
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

    //Player mockup
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 2., 1.))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.5, 0.3))),
        Transform::from_xyz(0., PLANET_RADIUS+1., 0.),
        Player{facing: Vec3::NEG_X},
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

fn enable_auto_indirect(
    info: Res<RenderAdapterInfo>,
    mut commands: Commands,
    cameras: Query<Entity, With<PanOrbitCamera>>,
) {
     let is_intel = info.vendor == 0x8086 || info.vendor == 32902;
     if is_intel {
        println!("Enabling NoIndirectDrawing for Intel GPUs to avoid issues with indirect drawing.");
        for entity in &cameras {
            commands.entity(entity).insert(NoIndirectDrawing);
        }
     }
}

fn toggle_wireframe(
    key: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<WireframeConfig>,
) {
    if key.just_pressed(KeyCode::Space) {
        config.global = !config.global;
        //info!("Wireframe mode: {}", if config.global { "ON" } else { "OFF" });
    }
}

fn follow_cam(
    mut pan_orbit_q: Query<&mut PanOrbitCamera>,
    player_q: Query<&Transform, With<Player>>
) {
    if let Ok(mut pan_orbit) = pan_orbit_q.single_mut() {
        if let Ok(player) = player_q.single() {
            pan_orbit.target_focus = player.translation;
            pan_orbit.force_update = true;
        }
    }
}

fn player_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut player_q: Query<(&mut Player, &mut Transform)>
) {
    if let Ok((mut player, mut player_transform)) = player_q.single_mut() {
        
        let mut pos = player_transform.translation;


        let up = pos.normalize();
        let forward = player.facing;
        let right = forward.cross(up);
        
        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) { input.y += 1.0; }
        if keys.pressed(KeyCode::KeyS) { input.y -= 1.0; }
        if keys.pressed(KeyCode::KeyD) { input.x += 1.0; }
        if keys.pressed(KeyCode::KeyA) { input.x -= 1.0; }
        input = input.normalize();

        let move_direction = (right * input.x + forward * input.y).normalize();

        //rotate around this axis to simulate movement
        let axis = up.cross(move_direction).normalize();
        let axis_len = axis.length();

        //prevents NAN issues when no input is given
        if axis_len > 1e-6 {
            let axis_n = axis / axis_len;
            let angle = (MOVE_SPEED * time.delta_secs()) / (PLANET_RADIUS);
            let rotation = Quat::from_axis_angle(axis_n, angle);
            pos = rotation * pos;

            //make sure we remain on the surface of the planet (corrects any floating point errors)
            pos = pos.normalize() * (PLANET_RADIUS+1.);

            //update position
            player_transform.translation = pos;

            //update rotation
            let up = pos.normalize();
            player_transform.rotation = Quat::from_rotation_arc(Vec3::Y, up);
            player.facing = up.cross(right);

        }

        

        


    }



    
}
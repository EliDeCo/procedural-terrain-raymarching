use std::f32::consts::FRAC_PI_4;

use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin}, platform::collections::HashMap, prelude::*, render::{render_resource::WgpuFeatures, settings::{RenderCreation, WgpuSettings}, RenderPlugin,}, window::{CursorGrabMode, PresentMode, PrimaryWindow}
};
//use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use iyes_perf_ui::prelude::*;
use bevy_panorbit_camera::{
    PanOrbitCameraPlugin, 
    PanOrbitCamera,
};
use terrain::TerrainStore;

mod terrain;

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
        .add_systems(Startup, (
            setup, 
            terrain::spawn_heightmap,
        ))
        .add_systems(Update, (
            grab_mouse,
            terrain::toggle_wireframe,
            control_ship,
            sync_camera_to_ship,
        ))
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
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, 
            -FRAC_PI_4, 
            -FRAC_PI_4, 
            0.0,
        )),
    ));

    // performance UI
    commands.spawn(
        (
            PerfUiEntryFPS::default(),
            PerfUiEntryFPSAverage::default(),
            PerfUiEntryFrameTime::default(),
            PerfUiEntryRenderCpuTime::default(),
            PerfUiEntryRenderGpuTime::default(),
            PerfUiEntryEntityCount::default(),
        )
    );
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

fn control_ship(
    input: Res<ButtonInput<KeyCode>>,
    mut ships: Query<&mut Transform, With<Ship>>,
) {
    let speed: f32 = 0.1;
    let mut direction = Vec2::new(0., 0.);
    if input.pressed(KeyCode::KeyW) {
        direction.y += speed;
    }
    if input.pressed(KeyCode::KeyS) {
        direction.y -= speed;
    }
    if input.pressed(KeyCode::KeyA) {
        direction.x += speed;
    }
    if input.pressed(KeyCode::KeyD) {
        direction.x -= speed;
    }
    for mut ship in &mut ships {
        ship.translation.x += direction.x;
        ship.translation.z += direction.y;
    }
}

fn sync_camera_to_ship(
    mut pan_orbit_q: Query<&mut PanOrbitCamera>, 
    cube_q: Query<&Transform, With<Ship>>) {
        if let Ok(mut pan_orbit) = pan_orbit_q.single_mut() {
            if let Ok(cube_tfm) = cube_q.single() {
                pan_orbit.target_focus = cube_tfm.translation;
                pan_orbit.force_update = true;
            }
        }
}
use std::f32::consts::FRAC_PI_4;

use bevy::{
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, PresentMode},
    pbr::wireframe::WireframePlugin,
    render::{render_resource::WgpuFeatures, settings::{RenderCreation, WgpuSettings}, RenderPlugin,},
};
use bevy_fly_camera::{FlyCamera, FlyCameraPlugin};
use iyes_perf_ui::prelude::*;

mod terrain;

fn main() {
    App::new()
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
        ))



        //sky color
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))

        .add_plugins(FlyCameraPlugin)
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
        ))
        .run();
}


fn setup(mut commands: Commands, mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    // camera
    commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 200.0, 800.0)
        ))
        .insert(FlyCamera {
            sensitivity: 8.0,
            ..default()
        });

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
    mut camera_controls: Query<&mut FlyCamera>,
) {
    if let Ok(mut primary_window) = q_windows.single_mut() {
        if mouse.just_pressed(MouseButton::Left) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::Locked;
            primary_window.cursor_options.visible = false;
            let center = Vec2::new(primary_window.width() / 2.0, primary_window.height() / 2.0);
            primary_window.set_cursor_position(Some(center));

            for mut options in camera_controls.iter_mut() {
                options.enabled = true; // Enable FlyCamera when mouse is pressed
            }
        }

        if key.just_pressed(KeyCode::Escape) {
            primary_window.cursor_options.grab_mode = CursorGrabMode::None;
            primary_window.cursor_options.visible = true;
            for mut options in camera_controls.iter_mut() {
                options.enabled = false; // Disable FlyCamera when Escape is pressed
            }
        }
    }
}

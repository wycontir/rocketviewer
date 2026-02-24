use serialport5::{self, SerialPortBuilder, SerialPort};
use std::io::{BufRead, BufReader};
use serde::{Deserialize, Serialize};
use bevy::prelude::*;
use bevy_egui::{
    EguiContextSettings, EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiStartupSet,
    EguiTextureHandle,
};


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .add_systems(Startup, (
            setup,
            setup_serial_monitor,
            setup_scene
        ))
        .add_systems(Update, (
            read_line,
            update_rocket_orientation
        ).chain())
        .run();
}

/*  
arduino sends this json string over serial port: (new lines added for readability but in reality it will be one line)
{
    "timestamp": 1234567890,
    "x": 0.0,
    "y": 0.0,
    "z": 0.0,
    "w": 1.0
}

Overview of app:
- on startup, app will show a menu with a dropdown of available serial ports and a start button to start the serial monitor
- when user selects a port and presses start, the app will start reading from the serial port
- app will display the 3d model of the rocket and update its orientation based on the quaternion data received from the serial port
- app will also display all data received as text on the side of the screen for debugging purposes and will write all data to a file with timestamps for later review

*/

//app state
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
enum AppState {
    #[default]
    Idle,
    Monitoring,
}


//struct counterpart to raw json received from arduino
#[derive(Serialize, Deserialize, Debug)]
struct ArduinoData {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    time: u32,
}
//resource struct that holds the most recent data from serial port
#[derive(Resource, Debug)]
struct CurrentData {
    quat: Quat,
    time: u32,
}
//resource struct that holds the serial port and reader for reading from the serial port
#[derive(Resource)]
struct SerialMonitorTools {
    reader: BufReader<SerialPort>,
    port_name: String,
    baud_rate: u32,
}
//marker component for rocket model
#[derive(Component)]
struct Rocket;

//sets up the 3d scene with a camera, light, and rocket model
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {

}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
        //create camera at about (5, 5, 5) looking at the origin
    //import 3d model of rocket and add it to the scene with a Rocket component
    //todo: add a bevy egui menu to select the serial port and baud rate and a start button to start the serial monitor system

    //create camera at about (5, 5, 5) looking at the origin with up being the Y axis
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    //Create directional light to illuminate the scene
    commands.spawn((
        DirectionalLight {
            color: Color::srgb( 0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0,).looking_at(Vec3::new(-0.15, -0.05, 0.25), -Vec3::Y),
    ));

    //Create rocket model and add the rocket component for updating
    commands.spawn((
        SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("Rocket Model High Poly.glb"),
        )),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Rocket,
    ));
}

//sets up serial monitor and current data resources, will be called when user presses start serial monitor button instead of on app startup once more user input implementation is done
fn setup_serial_monitor(mut commands: Commands) {
    //get list of serial ports
    //allow user to select which port to use
    //create port and reader and add them as a resource
    //todo implement serial port list and selection

    let port_name = "COM3"; //todo get list of ports and if only 1 port auto select it if multiple ports add a bevy egui dropdown to select the port
    let baud_rate = 9_600; //todo make this a user input in the app

    //create port and reader and add them as a resource todo: dont do this until user has selected the port and baud rate in app and pressed start serial monitor button
    let port = SerialPortBuilder::new()
        .baud_rate(baud_rate)
        .open(&port_name)
        .unwrap();
    let mut reader = BufReader::new(port);

    commands.insert_resource(SerialMonitorTools {
        reader,
        port_name: port_name.into(),
        baud_rate,
    });

    let current_data = CurrentData {
        quat: Quat::IDENTITY,
        time: 0,
    };
    commands.insert_resource(current_data);
}

//will run every update and will read the most recent line from serial port and update the currentdata resource
fn read_line(
    mut serial_tools: ResMut<SerialMonitorTools>,
    mut current_data: ResMut<CurrentData>,
) {
    let mut string = String::new();
    let read_line = serial_tools.reader.read_line(&mut string);
    //create struct that holds the data from serial port and serde_json the string into the struct
    //todo: make a file when starting the app that each incoming line will be written to along with a timestamp
    match read_line {
        Ok(_) => {
            let data_line: ArduinoData = serde_json::from_str(&string).unwrap();
            current_data.quat = Quat::from_xyzw(data_line.x, data_line.y, data_line.z, data_line.w);
            current_data.time = data_line.time;
            // dbg!(current_data);
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
        Err(e) => panic!("Error reading from serial port: {:?}", e),
    }
}

fn update_rocket_orientation(
    current_data: Res<CurrentData>,
    mut query: Query<&mut Transform, With<Rocket>>,
) {
    let quat = current_data.quat.clone();
    for mut transform in &mut query {
        transform.rotation = quat;
    }
}

//only used for proof of concept, will be removed in the future when the data is directly sent to the app instead of being printed to the console
#[allow(dead_code)]
fn serial_monitor() {
    let port_name = "COM3";
    let baud_rate = 9_600;

    let port = SerialPortBuilder::new()
        .baud_rate(baud_rate)
        .open(&port_name)
        .unwrap();

    let mut reader = BufReader::new(port);
    let mut string = String::new();
    loop {
        let read_line = reader.read_line(&mut string);
        match read_line {
            Ok(_) => {
                let quat: QuaternionTest = serde_json::from_str(&string).unwrap();
                dbg!(quat);
                string.clear();
            }
            Err(e) => (),
        }
    }
}
//only used for proof of concept, will be removed in the future when the data is directly sent to the app instead of being printed to the console
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
struct QuaternionTest {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

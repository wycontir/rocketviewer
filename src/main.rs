use serialport5::{self, SerialPortBuilder, SerialPort};
use std::io::{BufRead, BufReader, Read};
use serde::{Deserialize, Serialize};
use bevy::prelude::*;
use bevy_egui::{ EguiContexts, EguiPlugin, EguiPrimaryContextPass, EguiStartupSet, egui};

//constants
const DEFAULT_BAUD_RATE: u32 = 9_600;
const SUPPORTED_BAUD_RATES: [u32; 13] = [
    300, 
    600, 
    750, 
    1_200, 
    2_400, 
    4_800, 
    9_600, 
    19_200, 
    31_250, 
    38_400,
    57_600,
    74_880,
    115_200,
]; //list of baud rates the user can choose from
const ROCKET_MODEL_PATH: &str = "RocketLowPoly.glb";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin::default())
        .init_state::<AppState>() //initialize app state to idle
        .add_systems(PreStartup, setup_scene.before(EguiStartupSet::InitContexts)) //setup the 3d scene before egui contexts to avoid errors
        .add_systems(Startup, setup,)//set up serial port list and selection resources
        .add_systems(OnEnter(AppState::Monitoring), setup_serial_monitor) //when monitoring state is entered, set up the serial monitor
        .add_systems(Update, (
            read_line,
            update_rocket_orientation
        ).run_if(in_state(AppState::Monitoring))) //read data from serial port and update rocket model every frame
        .add_systems(EguiPrimaryContextPass, (
            ui_system_main,
        ))//main ui system for serial port selection, baud rate selection, and starting the serial monitor
        .add_systems(EguiPrimaryContextPass, (
            ui_system_monitor.run_if(in_state(AppState::Monitoring)),
        ))//ui system to display current telemetry data
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

//structs and resources
//app state
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
enum AppState {
    #[default]
    Idle,
    Monitoring,
}

//currently selected serial port and baud rate
#[derive(Resource)]
struct SerialMonitorSelection {
    port_name: String,
    baud_rate: u32,
}

//list of available serial ports
#[derive(Resource)]
struct SerialPortList {
    ports: Vec<String>,
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
    port: SerialPort,
}

//marker component for rocket model
#[derive(Component)]
struct Rocket;


// SETUP SYSTEMS

//main setup system
//sets up the list of serial ports and the default selections
fn setup(
    mut commands: Commands,
) {
    //get list of serial ports and a list of their names
    let ports = serialport5::available_ports().unwrap();
    let mut port_names = vec![];
    for port in ports {
        port_names.push(port.port_name);
    }

    let mut selection = SerialMonitorSelection {
        port_name: String::new(),
        baud_rate: DEFAULT_BAUD_RATE, //default baud rate, can be changed
    };

    match port_names.len() {
        0 => selection.port_name = "None".into(),
        1 => selection.port_name = port_names[0].clone(),
        _ => selection.port_name = port_names[0].clone(), //default to first port if multiple are available, user will be able to change this with a dropdown
    }

    commands.insert_resource(SerialPortList {
        ports: port_names,
    });
    commands.insert_resource(selection);
}

//scene setup system, will run before egui contexts are set up to avoid any errors
//sets up the 3d scene with a camera, light, and rocket model
fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    //import 3d model of rocket and add it to the scene with a Rocket component included for querying later

    //default camera distance from world origin
    let camera_distance = 1.0;

    //create camera at about (5, 5, 5) looking at the origin with up being the Y axis
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(camera_distance, camera_distance, camera_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    //Create directional light to illuminate the scene
    commands.spawn((
        DirectionalLight {
            color: Color::srgb( 0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(camera_distance, camera_distance, camera_distance,).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    //Create rocket model and add the rocket component for updating
    commands.spawn((
        SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset(ROCKET_MODEL_PATH),
        )),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Rocket,
    ));
}

//serial monitor setup system, will run when app state switches to monitoring
//sets up the serial monitor and port reader
fn setup_serial_monitor(
    mut commands: Commands,
    selected_port: Res<SerialMonitorSelection>,
) {
    //start port with the name and baud rate that is currently selected in the SerialMonitorSelection resource
    let port = SerialPortBuilder::new()
        .baud_rate(selected_port.baud_rate)
        .open(&selected_port.port_name)
        .unwrap();

    //insert serial monitor tools resource
    commands.insert_resource(SerialMonitorTools {
        port,
    });

    //insert current data resource with initial values
    let current_data = CurrentData {
        quat: Quat::IDENTITY,
        time: 0,
    };
    commands.insert_resource(current_data);
}


// UPDATE SYSTEMS

//data update system, runs every fram while in the monitoring state
//reads a line from the serial port, parses it into a json, and updates the current data resource with parsed data
fn read_line(
    mut serial_tools: ResMut<SerialMonitorTools>,
    mut current_data: ResMut<CurrentData>,
) {

    let mut buffer = [0; 128]; //128 byte buffer that the reader will fill every frame
    //optimally this entire thing would be its own thread started when monitoring state is entered but i dont know enough about rust multithreading for that so we will still lose some data since the frame time is about 60hz or 7-8ish ms
    match serial_tools.port.read(&mut buffer) {
        Ok(bytes_read) => {
            let data = String::from_utf8_lossy(&buffer[..bytes_read]);
            //lets get the last line and and serde it and dump the rest into the log file

            //pretend like we wrote that to a log file and continue
            let last_newline = data.rfind('\n').unwrap_or(0);
            let second_to_last_newline = data[..last_newline].rfind('\n').unwrap_or(std::usize::MAX);
            //sometimes we dont happen to catch enough data to get a full line and in that case we just wait until the next frame
            if last_newline == 0 || second_to_last_newline == std::usize::MAX {
                return;
            }
            let data_line: ArduinoData = serde_json::from_str(&data[second_to_last_newline+1..last_newline]).unwrap();
            current_data.quat = Quat::from_xyzw(data_line.x, data_line.y, data_line.z, data_line.w);
            current_data.time = data_line.time;
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
        Err(e) => panic!("Error reading from serial port: {:?}", e),
    }
}

//rocket model update system, runs every frame while in the monitoring state after the current data has been updated
//updates the orientation of the rocket model to match the most recent data received from the serial port
fn update_rocket_orientation(
    current_data: Res<CurrentData>,
    mut query: Query<&mut Transform, With<Rocket>>,
) {
    //get the most recent quat data
    let quat = current_data.quat.clone();
    //set the rocket models orientation to the quat received from the serial port
    //todo make it smoothly move to each orientation to make it look less choppy
    //can be done by making the newest orientation the target then slerp between current and target each frame
    for mut transform in &mut query {
        transform.rotation = quat;
    }
}


// UI SYSTEMS

//main ui system, runs every frame, allows user to select serial port, baud rate, and start the serial monitor
fn ui_system_main(
    mut contexts: EguiContexts,
    mut serial_port_list: ResMut<SerialPortList>,
    mut selection: ResMut<SerialMonitorSelection>,
    mut app_state: ResMut<NextState<AppState>>,
    current_app_state: Res<State<AppState>>,
) -> Result<(), BevyError> {
    let ctx = contexts.ctx_mut()?;
    //create floating window with dropdowns to select serial port and baud rate, and a button to start the serial monitor
    egui::Window::new("Serial Monitor Options")
        .default_width(200.0)
        .show(ctx, |ui| {
            //dropdown to select serial port
            ui.horizontal(|ui| {
                //check current selections
                let mut current_port = selection.port_name.clone();
                let mut current_baud_rate = selection.baud_rate;
                //serial port selection dropdown
                ui.label("Serial Port:");
                egui::ComboBox::from_label("")
                    .selected_text(current_port.clone())
                    .show_ui(ui, |ui| {
                        for port in &serial_port_list.ports {
                            ui.selectable_value(&mut current_port, port.clone(), port.clone());
                        }
                    });
                //change selected port if user selected a different one from the dropdown
                if current_port != selection.port_name {
                    println!("Switching selected port");
                    selection.port_name = current_port;
                }
                ui.label("at");
                //baud rate selection dropdown
                egui::ComboBox::from_label("Baud")
                    .selected_text(selection.baud_rate.to_string())
                    .show_ui(ui, |ui| {
                        for baud_rate in SUPPORTED_BAUD_RATES {
                            ui.selectable_value(&mut current_baud_rate, baud_rate, baud_rate.to_string());
                        }
                    });
                //change selected baud rate if user selected a different one from the dropdown
                if current_baud_rate != selection.baud_rate {
                    println!("Switching selected baud rate");
                    selection.baud_rate = current_baud_rate;
                }
            });
            ui.horizontal(|ui| {
                //refresh ports button
                if ui.button("Refresh Ports").clicked() {
                    //refresh list of ports
                    let ports = serialport5::available_ports().unwrap();
                    let mut port_names = vec![];
                    for port in ports {
                        port_names.push(port.port_name);
                    }
                    serial_port_list.ports = port_names;
                }
                //start serial monitor button
                if ui.button("Start Serial Monitor").clicked() {
                    //only start if a valid port is selected and if the app is not already monitoring
                    if selection.port_name != "None" && *current_app_state != AppState::Monitoring {
                        app_state.set(AppState::Monitoring);
                    } else {
                        println!("No valid port selected or already monitoring");
                    }
                }
            });
        });
    Ok(())
}

//data monitor ui system, runs every frame while in the monitoring state, displays the most recent data received from the serial port
fn ui_system_monitor(
    mut contexts: EguiContexts,
    current_data: Res<CurrentData>,
) -> Result<(), BevyError> {
    let ctx = contexts.ctx_mut()?;
    //create floating window that displays the most recent data received from the serial port
    egui::Window::new("Serial Monitor Data")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.label(format!("Time: {}", current_data.time));
            ui.label(format!("Quaternion: ({}, {}, {}, {})", current_data.quat.x, current_data.quat.y, current_data.quat.z, current_data.quat.w));
        });
    Ok(())
}


// DEPRACATED SYSTEMS AND STRUCTS

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
            Err(_) => (),
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

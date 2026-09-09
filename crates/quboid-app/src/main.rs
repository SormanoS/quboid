#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use quboid_app::Profile;

fn main() -> eframe::Result {
    let profile = Profile::base().expect("Quboid could not determine its configuration path");
    quboid_app::run(profile)
}

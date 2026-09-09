#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cuboid_app::Profile;

fn main() -> eframe::Result {
    let profile = Profile::base().expect("Cuboid could not determine its configuration path");
    cuboid_app::run(profile)
}

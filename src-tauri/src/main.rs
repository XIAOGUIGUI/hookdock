#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hookdock_windows_lib::run();
}

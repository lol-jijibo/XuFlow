// Tauri v2 application entry point
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    xuflow_desktop_lib::run()
}

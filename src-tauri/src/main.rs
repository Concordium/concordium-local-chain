// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


mod common;
mod unix;
mod utils;
mod views;
mod windows;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use utils::AppState;
use views::{
    install, install_genesis_creator, kill_chain, launch_template, list_chain_folders,
    verify_installation,
};


fn main() {
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .manage(app_state.clone())
        .setup(move |app| {
            // Get a reference to the main window
            let main_window = app.get_window("main").unwrap();

            // Store the window reference in the app state
            let mut state = app_state.lock().unwrap();
            state.main_window = Some(main_window);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            install,
            verify_installation,
            install_genesis_creator,
            launch_template,
            list_chain_folders,
            kill_chain
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
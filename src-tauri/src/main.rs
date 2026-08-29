// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use bearpad_lib;

fn main() {
    // Set up panic logging to a file in the temp directory
    let log_dir = std::env::temp_dir();
    let log_file_path = log_dir.join("bearpad_panic.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .unwrap_or_else(|_| {
            // Fallback: try to create the file
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_file_path)
                .unwrap_or_else(|_| {
                    // If we still can't open/create the file, we'll skip logging
                    // and just hope the app runs. We create a dummy file that we drop.
                    let _ = std::fs::File::create(&log_file_path);
                    std::fs::File::create("/dev/null").unwrap()
                })
        });
    let file = Arc::new(Mutex::new(file));

    let _ = writeln!(file.lock().unwrap(), "Application started at {:?}", SystemTime::now());

    // Install a panic hook that writes to our log file
    let file_clone = Arc::clone(&file);
    panic::set_hook(Box::new(move |panic_info| {
        if let Ok(mut f) = file_clone.lock() {
            let _ = writeln!(f, "Panic: {}", panic_info);
        }
    }));

    // Run the Tauri application
    bearpad_lib::run();
}
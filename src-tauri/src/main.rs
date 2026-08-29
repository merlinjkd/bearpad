// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::time::SystemTime;
use std::path::PathBuf;

fn main() {
    // Set up panic logging to a file in the temp directory
    let log_dir = std::env::temp_dir();
    let log_file_path = log_dir.join("bearpad_panic.log");
    let mut file = OpenOptions::new()
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

    let _ = writeln!(file, "Application started at {:?}", SystemTime::now());

    // Install a panic hook that writes to our log file
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = writeln!(file, "Panic: {}", panic_info);
        hook(panic_info);
    }));

    // Run the Tauri application
    if let Err(e) = bearpad_lib::run() {
        let _ = writeln!(file, "Error running Tauri app: {}", e);
        // Optionally, we could show an error dialog here, but for now just exit.
        std::process::exit(1);
    }
}

//! Executable entry point.
//!
//! Deliberately thin: everything lives in the library so the application can be
//! built and inspected as a library target too.

// Prevents a console window from opening alongside the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    midi_monitor_lib::run();
}

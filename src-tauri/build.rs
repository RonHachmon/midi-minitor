//! Tauri's build-time code generation.
//!
//! Emits the context that `tauri::generate_context!` expands, from
//! `tauri.conf.json` and the capability files beside it.

fn main() {
    tauri_build::build();
}

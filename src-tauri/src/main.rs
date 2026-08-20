// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args_os().any(|argument| argument == "--cemm-sidecar-service") {
        std::process::exit(cemm_lib::service::run_stdio_service());
    }

    cemm_lib::run();
}

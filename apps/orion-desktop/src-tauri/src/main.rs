#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{Manager, State};

pub struct AppState {
    pub core_process: Mutex<Option<Child>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub endpoint: String,
}

fn spawn_orion_core() -> std::io::Result<Child> {
    let exe = std::env::current_exe()?;
    let bin_dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let target_triple = format!(
        "{}-{}-{}",
        std::env::consts::ARCH,
        match std::env::consts::OS {
            "linux" => "unknown-linux",
            "macos" => "apple-darwin",
            "windows" => "pc-windows",
            _ => "unknown",
        },
        std::env::consts::OS
    );

    let candidates = [
        bin_dir.join(format!("orion-server-{}", target_triple)),
        bin_dir.join("orion-server"),
        bin_dir.join("../orion-server"),
        bin_dir.join("../../orion-server/debug/orion-server"),
    ];
    let core_bin = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| std::path::PathBuf::from("orion-server"));

    tracing::info!("spawning orion-core at {:?}", core_bin);

    Command::new(&core_bin)
        .env("ORION_PORT", "7337")
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

#[tauri::command]
fn core_status(state: State<'_, AppState>) -> CoreStatus {
    let mut guard = state.core_process.lock().unwrap();
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                *guard = None;
                CoreStatus {
                    running: false,
                    pid: None,
                    endpoint: "http://127.0.0.1:7337".into(),
                }
            }
            Ok(None) => CoreStatus {
                running: true,
                pid: Some(child.id()),
                endpoint: "http://127.0.0.1:7337".into(),
            },
            Err(_) => CoreStatus {
                running: false,
                pid: None,
                endpoint: "http://127.0.0.1:7337".into(),
            },
        }
    } else {
        CoreStatus {
            running: false,
            pid: None,
            endpoint: "http://127.0.0.1:7337".into(),
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let core = spawn_orion_core().ok();
    std::thread::sleep(std::time::Duration::from_millis(800));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            core_process: Mutex::new(core),
        })
        .invoke_handler(tauri::generate_handler![core_status])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.app_handle().try_state::<AppState>() {
                    if let Ok(mut guard) = state.core_process.lock() {
                        if let Some(mut child) = guard.take() {
                            let _ = child.kill();
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

//! Briefly retain closed WebViews; reclaim hidden windows after inactivity.
use super::*;
use std::{collections::HashMap, time::Instant};
const IDLE: Duration = Duration::from_secs(45);
#[derive(Default)]
pub struct Surfaces(Mutex<HashMap<String, Instant>>);
pub fn touch(app: &tauri::AppHandle, label: &str) {
    if let Ok(mut entries) = app.state::<Surfaces>().0.lock() {
        entries.insert(label.into(), Instant::now());
    }
}
pub fn retain(app: &tauri::AppHandle, window: &tauri::WebviewWindow) {
    touch(app, window.label());
    let handle = app.clone();
    let watched = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            touch(&handle, watched.label());
            if watched.label() == "explanation" {
                crate::screen_helper::dismiss_presentation(&handle);
                crate::pet::explaining(&handle, false);
                crate::pet::watch_bubble(&handle, 0);
            }
            let _ = watched.hide();
        }
        tauri::WindowEvent::Focused(true) => touch(&handle, watched.label()),
        _ => {}
    });
}
pub fn start(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(5));
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let state = handle.state::<Surfaces>();
            let entries = state.0.lock().unwrap_or_else(|e| e.into_inner()).clone();
            for (label, last) in entries {
                let Some(window) = handle.get_webview_window(&label) else {
                    state
                        .0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&label);
                    continue;
                };
                if window.is_visible().unwrap_or(true) {
                    touch(&handle, &label);
                    continue;
                }
                if last.elapsed() < IDLE {
                    continue;
                }
                // Native destruction can invoke window callbacks: never hold our
                // mutex across it, and retry a failed destroy on the next sweep.
                if window.destroy().is_ok() {
                    state
                        .0
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&label);
                }
            }
        });
    });
}

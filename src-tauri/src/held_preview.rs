//! On-demand bounded previews; no network, model calls, or document execution.
use super::*;
use base64::Engine;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
#[derive(Default)]
pub struct Preview {
    value: Mutex<Option<(PathBuf, Value)>>,
    generation: AtomicU64,
}
fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32, &'static str)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.len() >= 24 {
        if bytes.windows(4).any(|chunk| chunk == b"acTL") {
            return None;
        }
        return Some((
            u32::from_be_bytes(bytes[16..20].try_into().ok()?),
            u32::from_be_bytes(bytes[20..24].try_into().ok()?),
            "image/png",
        ));
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 26 {
        let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs();
        let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs();
        return Some((w, h, "image/bmp"));
    }
    if bytes.starts_with(&[0xff, 0xd8]) {
        let mut offset = 2;
        while offset + 4 <= bytes.len() {
            if bytes[offset] != 0xff {
                return None;
            }
            while offset < bytes.len() && bytes[offset] == 0xff {
                offset += 1;
            }
            let marker = *bytes.get(offset)?;
            offset += 1;
            if marker == 0xd9 || marker == 0xda {
                return None;
            }
            if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
                continue;
            }
            let len = u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
            if len < 2 || offset + len > bytes.len() {
                return None;
            }
            if matches!(
                marker,
                0xc0 | 0xc1
                    | 0xc2
                    | 0xc3
                    | 0xc5
                    | 0xc6
                    | 0xc7
                    | 0xc9
                    | 0xca
                    | 0xcb
                    | 0xcd
                    | 0xce
                    | 0xcf
            ) {
                if len < 8 {
                    return None;
                }
                return Some((
                    u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().ok()?) as u32,
                    u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().ok()?) as u32,
                    "image/jpeg",
                ));
            }
            offset += len;
        }
    }
    None
}
fn read(path: &Path) -> Result<Value, String> {
    if let Some(reference) = crate::held_refs::get(path) {
        return Ok(
            json!({"name":reference.label,"kind":if reference.link{"link"}else{"text"},"text":reference.text}),
        );
    }
    let meta = std::fs::metadata(path).map_err(err)?;
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    if meta.is_dir() {
        return Ok(
            json!({"name":name,"kind":"details","message":"Folder · open to browse its contents."}),
        );
    }
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp") && meta.len() <= 8 * 1024 * 1024 {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(err)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() <= 8 * 1024 * 1024 {
            if let Some((width, height, mime)) = image_dimensions(&bytes) {
                if width > 0
                    && height > 0
                    && width <= 4096
                    && height <= 4096
                    && u64::from(width) * u64::from(height) <= 4_000_000
                {
                    return Ok(
                        json!({"name":name,"kind":"image","size":meta.len(),"image":format!("data:{mime};base64,{}",base64::engine::general_purpose::STANDARD.encode(&bytes))}),
                    );
                }
            }
        }
    }
    if matches!(
        ext.as_str(),
        "txt"
            | "md"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "json"
            | "csv"
            | "log"
            | "html"
            | "css"
            | "xml"
            | "svg"
            | "c"
            | "cpp"
            | "java"
            | "toml"
            | "yaml"
            | "yml"
    ) {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(err)?
            .take(64 * 1024)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if !bytes.contains(&0) {
            return Ok(
                json!({"name":name,"kind":"text","size":meta.len(),"text":String::from_utf8_lossy(&bytes),"truncated":meta.len()>bytes.len() as u64}),
            );
        }
    }
    Ok(
        json!({"name":name,"kind":"details","size":meta.len(),"message":"Quick preview isn't available for this file. Open it in its usual app."}),
    )
}
pub fn clear(app: &tauri::AppHandle) {
    let state = app.state::<Preview>();
    state.generation.fetch_add(1, Ordering::SeqCst);
    *state.value.lock().unwrap_or_else(|e| e.into_inner()) = None;
    let _ = app.emit("held-preview-changed", ());
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("held-preview") {
            let _ = window.close();
        }
    });
}
pub fn open(app: tauri::AppHandle, path: PathBuf) {
    let generation = app
        .state::<Preview>()
        .generation
        .fetch_add(1, Ordering::SeqCst)
        + 1;
    crate::surfaces::touch(&app, "held-preview");
    tauri::async_runtime::spawn_blocking(move || {
        let value = read(&path)
            .unwrap_or_else(|error| json!({"name":"Preview","kind":"details","error":error}));
        {
            let state = app.state::<Preview>();
            let mut saved = state.value.lock().unwrap_or_else(|e| e.into_inner());
            if state.generation.load(Ordering::SeqCst) != generation {
                return;
            }
            *saved = Some((path, value));
        }
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if handle.state::<Preview>().generation.load(Ordering::SeqCst) != generation {
                return;
            }
            let created = handle.get_webview_window("held-preview").is_none();
            let window = if let Some(window) = handle.get_webview_window("held-preview") {
                Ok(window)
            } else {
                tauri::WebviewWindowBuilder::new(
                    &handle,
                    "held-preview",
                    tauri::WebviewUrl::App("preview.html".into()),
                )
                .title("Kitty · Preview")
                .inner_size(420.0, 440.0)
                .min_inner_size(300.0, 220.0)
                .visible(false)
                .always_on_top(true)
                .build()
            };
            if let Ok(window) = window {
                if created {
                    crate::surfaces::retain(&handle, &window);
                    let on_close = handle.clone();
                    window.on_window_event(move |event| {
                        if matches!(
                            event,
                            tauri::WindowEvent::CloseRequested { .. }
                                | tauri::WindowEvent::Destroyed
                        ) {
                            on_close
                                .state::<Preview>()
                                .generation
                                .fetch_add(1, Ordering::SeqCst);
                            *on_close
                                .state::<Preview>()
                                .value
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = None;
                            let _ = on_close.emit("held-preview-changed", ());
                        }
                    });
                }
                let _ = handle.emit("held-preview-changed", ());
                let _ = window.show();
                let _ = window.set_focus();
            }
        });
    });
}
#[tauri::command]
pub fn held_preview_read(state: tauri::State<Preview>) -> Value {
    state
        .value
        .lock()
        .ok()
        .and_then(|value| value.as_ref().map(|(_, data)| data.clone()))
        .unwrap_or(Value::Null)
}
#[tauri::command]
pub fn held_preview_open(state: tauri::State<Preview>) -> Result<(), String> {
    let path = state
        .value
        .lock()
        .map_err(err)?
        .as_ref()
        .map(|(path, _)| path.clone())
        .ok_or("Preview closed")?;
    if let Some(reference) = crate::held_refs::get(&path) {
        if reference.link {
            let url = tauri::Url::parse(&reference.text).map_err(err)?;
            if !matches!(url.scheme(), "http" | "https") {
                return Err("Unsupported link".into());
            }
            let text: Vec<u16> = url.as_str().encode_utf16().chain(Some(0)).collect();
            unsafe {
                use windows::{
                    core::{w, PCWSTR},
                    Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
                };
                let result = ShellExecuteW(
                    None,
                    w!("open"),
                    PCWSTR(text.as_ptr()),
                    None,
                    None,
                    SW_SHOWNORMAL,
                );
                if result.0 as isize <= 32 {
                    return Err("Windows could not open this link.".into());
                }
            }
            return Ok(());
        }
    }
    super::open_path(path.display().to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_headers_are_bounded_and_not_guessed() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend([0; 8]);
        png.extend(123u32.to_be_bytes());
        png.extend(45u32.to_be_bytes());
        assert_eq!(image_dimensions(&png), Some((123, 45, "image/png")));
        png.extend(b"acTL");
        assert!(image_dimensions(&png).is_none());
        assert!(image_dimensions(b"not an image").is_none());
    }
    #[test]
    fn text_previews_never_execute_html_and_are_capped() {
        let root = std::env::temp_dir().join(format!("buddy-preview-{}", new_id()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("example.html");
        std::fs::write(
            &path,
            format!("<script>alert(1)</script>{}", "a".repeat(100000)),
        )
        .unwrap();
        let value = read(&path).unwrap();
        assert_eq!(value["kind"], "text");
        assert_eq!(value["truncated"], true);
        assert!(value["text"].as_str().unwrap().len() <= 65536);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}

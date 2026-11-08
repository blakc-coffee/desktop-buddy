use super::*;
use crate::commands::Destination;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    id: u64,
    destination: Destination,
}
#[derive(Default)]
pub struct Navigation(Mutex<(u64, Option<Request>)>);
impl Navigation {
    fn queue(&self, destination: Destination) -> Result<Request, String> {
        let mut state = self.0.lock().map_err(err)?;
        state.0 += 1;
        let request = Request {
            id: state.0,
            destination,
        };
        state.1 = Some(request.clone());
        Ok(request)
    }
    fn acknowledge(&self, id: u64) -> Result<(), String> {
        let mut state = self.0.lock().map_err(err)?;
        if state.1.as_ref().is_some_and(|request| request.id == id) {
            state.1 = None;
        }
        Ok(())
    }
}
#[tauri::command]
pub fn panel_navigation(state: tauri::State<Navigation>) -> Result<Option<Request>, String> {
    Ok(state.0.lock().map_err(err)?.1.clone())
}
#[tauri::command]
pub fn acknowledge_navigation(state: tauri::State<Navigation>, id: u64) -> Result<(), String> {
    state.acknowledge(id)
}
pub fn open(app: tauri::AppHandle, destination: Destination) {
    if !crate::features::destination_enabled(destination) { return; }
    if destination == Destination::Codex { chats(app); return; }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(request) = handle.state::<Navigation>().queue(destination) {
            show_panel(&handle);
            let _ = handle.emit("panel-navigation", request);
        }
    });
}
pub fn todo(app: tauri::AppHandle, palette: crate::pet_art::Palette) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let css = |color: windows::Win32::Foundation::COLORREF| format!("#{:02x}{:02x}{:02x}", color.0 & 255, (color.0 >> 8) & 255, (color.0 >> 16) & 255);
        let script = format!("window.__BUDDY_SURFACE__='todo';window.__BUDDY_PALETTE__={};", serde_json::json!({"surface":css(palette.surface),"ink":css(palette.ink),"accent":css(palette.accent),"selection":css(palette.selection),"outline":css(palette.outline)}));
        if let Some(window) = handle.get_webview_window("todo") {
            let _ = window.eval(&format!("{}window.dispatchEvent(new Event('buddy-palette'));", script));
            let _ = window.show();
            let _ = window.set_focus();
            return;
        }
        if let Err(error) = tauri::WebviewWindowBuilder::new(&handle, "todo", tauri::WebviewUrl::App("index.html".into()))
            .title("Todo").inner_size(360.0, 420.0).min_inner_size(300.0, 280.0)
            .initialization_script(&script).build() {
            if let Ok(mut warning) = handle.state::<Shared>().warning.lock() { *warning = Some(format!("Could not open Todo: {error}")); }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_acknowledgements_never_remove_newer_navigation() {
        let state = Navigation::default();
        let first = state.queue(Destination::Ask).unwrap();
        let second = state.queue(Destination::Settings).unwrap();
        state.acknowledge(first.id).unwrap();
        assert_eq!(state.0.lock().unwrap().1.as_ref().unwrap().id, second.id);
        state.acknowledge(second.id).unwrap();
        assert!(state.0.lock().unwrap().1.is_none());
    }
}

pub fn chats(app:tauri::AppHandle) {
    if !crate::features::CODEX_CHATS { return; }
    let handle=app.clone();
    let _=app.run_on_main_thread(move || {
        let window=if let Some(window)=handle.get_webview_window("chats") {Ok(window)} else {
            tauri::WebviewWindowBuilder::new(&handle,"chats",tauri::WebviewUrl::App("index.html?surface=chats".into()))
                .title("Kitty · Codex chats").inner_size(420.0,480.0).min_inner_size(280.0,240.0)
                .visible(false).decorations(false).transparent(true).shadow(false).always_on_top(true).skip_taskbar(true).build()
        };
        if let Ok(window)=window {
            if let Some(control)=handle.try_state::<Arc<crate::pet::Control>>() {
                if let Some((x,y,side,_))=crate::pet::explanation_position(&control) {let _=window.set_position(tauri::PhysicalPosition::new(x,y));let _=window.eval(&format!("document.documentElement.dataset.tail=\"{}\"",side));}
            }
            let _=window.show();let _=window.set_focus();
        }
    });
}

#[tauri::command]
pub fn bubble_anchor(app:tauri::AppHandle)->serde_json::Value {
    app.try_state::<Arc<crate::pet::Control>>().and_then(|control| crate::pet::explanation_position(&control))
        .map(|(_,_,side,tail)|serde_json::json!({"side":side,"tail":tail})).unwrap_or(serde_json::json!({"side":"left","tail":50}))
}

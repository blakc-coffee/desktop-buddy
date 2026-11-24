//! Hides the pet while a fullscreen app (a game, slides, full-screen video) is in front.
//! Windows reports that state through SHQueryUserNotificationState; it is re-checked only when
//! the foreground window changes or resizes (WinEvent hooks), so nothing polls while idle.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Accessibility::{SetWinEventHook, HWINEVENTHOOK},
        Shell::SHQueryUserNotificationState,
        WindowsAndMessaging::{
            GetForegroundWindow, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, OBJID_WINDOW,
            WINEVENT_OUTOFCONTEXT,
        },
    },
};

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
static HIDDEN: AtomicBool = AtomicBool::new(false);

/// QUNS_BUSY (2, borderless fullscreen such as F11 video), QUNS_RUNNING_D3D_FULL_SCREEN (3) and
/// QUNS_PRESENTATION_MODE (4) mean the user should not see a pet on top.
pub fn hide_for(state: i32) -> bool {
    matches!(state, 2..=4)
}

/// Must run on a thread with a message loop (Tauri's main thread during setup).
pub fn install(app: tauri::AppHandle) {
    if APP.set(app).is_err() {
        return;
    }
    unsafe {
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(changed), 0, 0, WINEVENT_OUTOFCONTEXT);
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(changed), 0, 0, WINEVENT_OUTOFCONTEXT);
    }
}

unsafe extern "system" fn changed(_: HWINEVENTHOOK, event: u32, hwnd: HWND, object: i32, child: i32, _: u32, _: u32) {
    // Location changes fire for carets and every window; only the foreground window's own frame counts.
    if event == EVENT_OBJECT_LOCATIONCHANGE && (object != OBJID_WINDOW.0 || child != 0 || hwnd != GetForegroundWindow()) {
        return;
    }
    let Ok(state) = SHQueryUserNotificationState() else { return };
    let hide = hide_for(state.0);
    if HIDDEN.swap(hide, Ordering::SeqCst) != hide {
        if let Some(app) = APP.get() {
            crate::pet::set_fullscreen(app, hide);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fullscreen_states_hide_the_pet() {
        assert!(hide_for(2) && hide_for(3) && hide_for(4));
        assert!(!hide_for(1) && !hide_for(5) && !hide_for(0));
    }
}

//! Explicit, one-shot screen-area picker. Never created during background work.
use super::*;
use std::cell::RefCell;
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[derive(Default)]
struct Selection {
    start: Option<POINT>,
    end: POINT,
    result: Option<Result<RECT, String>>,
}
thread_local! {static SELECT:RefCell<Selection>=RefCell::new(Selection::default());}
fn rectangle(start: POINT, end: POINT) -> Result<RECT, String> {
    let r = RECT {
        left: start.x.min(end.x),
        top: start.y.min(end.y),
        right: start.x.max(end.x),
        bottom: start.y.max(end.y),
    };
    if r.right - r.left < 12 || r.bottom - r.top < 12 {
        Err("Select a larger area, or capture the display.".into())
    } else {
        Ok(r)
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let point = || POINT {
        x: lp.0 as i16 as i32,
        y: (lp.0 >> 16) as i16 as i32,
    };
    match msg {
        WM_LBUTTONDOWN => {
            SELECT.with(|s| {
                let mut s = s.borrow_mut();
                s.start = Some(point());
                s.end = point();
            });
            SetCapture(hwnd);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            SELECT.with(|s| s.borrow_mut().end = point());
            let _ = InvalidateRect(Some(hwnd), None, true);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            SELECT.with(|s| {
                let mut s = s.borrow_mut();
                s.result = Some(
                    s.start
                        .ok_or("No area selected.".into())
                        .and_then(|start| rectangle(start, point())),
                );
            });
            LRESULT(0)
        }
        WM_KEYDOWN if wp.0 == VK_ESCAPE.0 as usize => {
            SELECT.with(|s| s.borrow_mut().result = Some(Err("Area capture cancelled.".into())));
            LRESULT(0)
        }
        WM_TIMER => {
            SELECT.with(|s| {
                s.borrow_mut().result = Some(Err(
                    "Area selection timed out. Capture again when ready.".into(),
                ))
            });
            LRESULT(0)
        }
        WM_ACTIVATEAPP if wp.0 == 0 => {
            SELECT.with(|s| {
                s.borrow_mut().result =
                    Some(Err("Area selection cancelled after focus changed.".into()))
            });
            LRESULT(0)
        }
        WM_CLOSE => {
            SELECT.with(|s| s.borrow_mut().result = Some(Err("Area capture cancelled.".into())));
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(dc, &rect, HBRUSH(GetStockObject(BLACK_BRUSH).0));
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00ffffff));
            SetBkColor(dc, COLORREF(0x00302b28));
            let font = CreateFontW(
                -18,
                0,
                0,
                0,
                500,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                DEFAULT_PITCH.0 as u32,
                w!("Segoe UI"),
            );
            let oldfont = SelectObject(dc, font.into());
            let mut text: Vec<_> = "Drag to select an area · Esc to cancel"
                .encode_utf16()
                .collect();
            let mut label = RECT {
                left: 24,
                top: 24,
                right: rect.right - 24,
                bottom: 70,
            };
            DrawTextW(dc, &mut text, &mut label, DT_SINGLELINE | DT_NOPREFIX);
            SELECT.with(|s| {
                let s = s.borrow();
                if let Some(start) = s.start {
                    if let Ok(r) = rectangle(start, s.end) {
                        let pen = CreatePen(PS_SOLID, 3, COLORREF(0x009ac1f2));
                        let old = SelectObject(dc, pen.into());
                        let b = SelectObject(dc, GetStockObject(NULL_BRUSH));
                        let _ = Rectangle(dc, r.left, r.top, r.right, r.bottom);
                        SelectObject(dc, b);
                        SelectObject(dc, old);
                        let _ = DeleteObject(pen.into());
                    }
                }
            });
            SelectObject(dc, oldfont);
            let _ = DeleteObject(font.into());
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
pub unsafe fn choose(display: RECT) -> Result<RECT, String> {
    SELECT.with(|s| *s.borrow_mut() = Selection::default());
    let instance = GetModuleHandleW(None).map_err(err)?;
    let class = w!("BuddyScreenAreaPicker");
    RegisterClassW(&WNDCLASSW {
        lpfnWndProc: Some(proc),
        hInstance: instance.into(),
        lpszClassName: class,
        hCursor: LoadCursorW(None, IDC_CROSS).map_err(err)?,
        ..Default::default()
    });
    let window = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
        class,
        w!("Choose screen area"),
        WS_POPUP,
        display.left,
        display.top,
        display.right - display.left,
        display.bottom - display.top,
        None,
        None,
        Some(instance.into()),
        None,
    )
    .map_err(err)?;
    // Almost-transparent black remains hit-testable, unlike a color-key hole.
    let _ = SetLayeredWindowAttributes(window, COLORREF(0), 110, LWA_ALPHA);
    SetTimer(Some(window), 1, 120000, None);
    let _ = ShowWindow(window, SW_SHOW);
    let _ = SetForegroundWindow(window);
    let _ = SetFocus(Some(window));
    let mut message = MSG::default();
    let result = loop {
        if GetMessageW(&mut message, None, 0, 0).0 <= 0 {
            break Err("Area capture cancelled.".into());
        }
        let _ = TranslateMessage(&message);
        DispatchMessageW(&message);
        if let Some(result) = SELECT.with(|s| s.borrow_mut().result.take()) {
            break result;
        }
    };
    let _ = DestroyWindow(window);
    std::thread::sleep(Duration::from_millis(100));
    result.map(|r| RECT {
        left: r.left + display.left,
        top: r.top + display.top,
        right: r.right + display.left,
        bottom: r.bottom + display.top,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_handles_reverse_drags_and_rejects_tiny_regions() {
        let r = rectangle(POINT { x: 100, y: 80 }, POINT { x: 10, y: 20 }).unwrap();
        assert_eq!((r.left, r.top, r.right, r.bottom), (10, 20, 100, 80));
        assert!(rectangle(POINT { x: 1, y: 1 }, POINT { x: 4, y: 4 }).is_err());
    }
}

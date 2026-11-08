//! Click-through file labels owned by the native pet, with no idle animation.
use super::*;
use std::cell::RefCell;
use windows::{
    core::w,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Folder,
    Pdf,
    Text,
    Sheet,
    Image,
    Code,
    Archive,
    File,
}
fn kind(path: &Path) -> Kind {
    if path.is_dir() {
        return Kind::Folder;
    }
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => Kind::Pdf,
        "txt" | "md" | "rtf" | "doc" | "docx" => Kind::Text,
        "xls" | "xlsx" | "xlsm" | "csv" | "ods" => Kind::Sheet,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => Kind::Image,
        "rs" | "py" | "js" | "ts" | "tsx" | "cpp" | "c" | "java" | "json" => Kind::Code,
        "zip" | "rar" | "7z" | "tar" | "gz" => Kind::Archive,
        _ => Kind::File,
    }
}
struct Node {
    window: HWND,
    path: PathBuf,
    label: String,
    kind: Kind,
}
#[derive(Default)]
struct Halo {
    nodes: Vec<Node>,
    palette: Option<crate::pet_art::Palette>,
}
thread_local! { static HALO: RefCell<Halo> = RefCell::new(Halo::default()); }

// Two flanking columns when possible, otherwise a compact grid on the roomy side.
fn positions(pet: RECT, work: RECT, count: usize, scale: i32) -> Vec<RECT> {
    let width = (160 * scale / 96).min((work.right - work.left).max(1));
    let height = 32 * scale / 96;
    let gap = 7 * scale / 96;
    let left = pet.left - work.left;
    let right = work.right - pet.right;
    let both = left >= width + gap && right >= width + gap;
    let rows_max = ((work.bottom - work.top) / (height + gap)).max(1) as usize;
    let split = if both { count.div_ceil(2) } else { count };
    let rows = split.min(rows_max).max(1);
    let total_height = rows as i32 * (height + gap) - gap;
    let top = ((pet.top + pet.bottom - total_height) / 2)
        .clamp(work.top, (work.bottom - total_height).max(work.top));
    (0..count)
        .map(|index| {
            let (on_left, item) = if both {
                (index % 2 == 0, index / 2)
            } else {
                (left >= right, index)
            };
            let column = item / rows;
            let row = item % rows;
            let x = if on_left {
                pet.left - gap - width - (column as i32) * (width + gap)
            } else {
                pet.right + gap + (column as i32) * (width + gap)
            };
            let x = x.clamp(work.left, (work.right - width).max(work.left));
            RECT {
                left: x,
                top: top + row as i32 * (height + gap),
                right: x + width,
                bottom: top + row as i32 * (height + gap) + height,
            }
        })
        .collect()
}
unsafe fn draw(
    dc: HDC,
    rect: RECT,
    label: &str,
    kind: Kind,
    palette: crate::pet_art::Palette,
    dpi: i32,
) {
    let s = |n: i32| n * dpi / 96;
    let surface = CreateSolidBrush(palette.surface);
    let outline = CreatePen(PS_SOLID, 1, palette.outline);
    let oldb = SelectObject(dc, surface.into());
    let oldp = SelectObject(dc, outline.into());
    let _ = RoundRect(
        dc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        s(12),
        s(12),
    );
    let color = match kind {
        Kind::Pdf => COLORREF(0x0099a0ff),
        Kind::Sheet => COLORREF(0x00addb91),
        _ => palette.accent,
    };
    let pen = CreatePen(PS_SOLID, s(1).max(1), color);
    SelectObject(dc, pen.into());
    SelectObject(dc, GetStockObject(NULL_BRUSH));
    let x = rect.left + s(10);
    let y = rect.top + s(7);
    let line = |a, b, c, d| {
        let _ = MoveToEx(dc, x + s(a), y + s(b), None);
        let _ = LineTo(dc, x + s(c), y + s(d));
    };
    if kind == Kind::Folder {
        line(0, 4, 0, 17);
        line(0, 17, 19, 17);
        line(19, 17, 19, 4);
        line(19, 4, 8, 4);
        line(8, 4, 5, 1);
        line(5, 1, 0, 1);
        line(0, 1, 0, 4);
    } else {
        line(2, 0, 13, 0);
        line(13, 0, 18, 5);
        line(18, 5, 18, 18);
        line(18, 18, 2, 18);
        line(2, 18, 2, 0);
        line(13, 0, 13, 5);
        line(13, 5, 18, 5);
        match kind {
            Kind::Sheet => {
                for y in [8, 12] {
                    line(5, y, 15, y);
                }
                line(10, 7, 10, 16);
            }
            Kind::Image => {
                line(5, 15, 9, 10);
                line(9, 10, 12, 13);
                line(12, 13, 15, 9);
            }
            Kind::Code => {
                line(8, 8, 5, 11);
                line(5, 11, 8, 14);
                line(12, 8, 15, 11);
                line(15, 11, 12, 14);
            }
            _ => {
                line(5, 9, 15, 9);
                line(5, 13, 13, 13);
            }
        }
    }
    SelectObject(dc, oldp);
    SelectObject(dc, oldb);
    let _ = DeleteObject(pen.into());
    let _ = DeleteObject(outline.into());
    let _ = DeleteObject(surface.into());
    let font = CreateFontW(
        -s(12),
        0,
        0,
        0,
        400,
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
    let old = SelectObject(dc, font.into());
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, palette.ink);
    let mut bounds = RECT {
        left: rect.left + s(38),
        top: rect.top,
        right: rect.right - s(9),
        bottom: rect.bottom,
    };
    let mut text: Vec<_> = label.encode_utf16().collect();
    DrawTextW(
        dc,
        &mut text,
        &mut bounds,
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    SelectObject(dc, old);
    let _ = DeleteObject(font.into());
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            FillRect(dc, &rect, HBRUSH(GetStockObject(BLACK_BRUSH).0));
            HALO.with(|halo| {
                let halo = halo.borrow();
                if let (Some(node), Some(palette)) = (
                    halo.nodes.iter().find(|node| node.window == hwnd),
                    halo.palette,
                ) {
                    draw(
                        dc,
                        rect,
                        &node.label,
                        node.kind,
                        palette,
                        GetDpiForWindow(hwnd).max(96) as i32,
                    );
                }
            });
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
pub unsafe fn clear() {
    let windows = HALO.with(|h| {
        h.borrow_mut()
            .nodes
            .drain(..)
            .map(|n| n.window)
            .collect::<Vec<_>>()
    });
    for window in windows {
        let _ = DestroyWindow(window);
    }
}
pub unsafe fn sync(owner: HWND, paths: &[PathBuf], palette: crate::pet_art::Palette) {
    if paths.len() < 2 || !IsWindowVisible(owner).as_bool() {
        clear();
        return;
    }
    let changed = HALO.with(|h| {
        let h = h.borrow();
        h.nodes.len() != paths.len() || h.nodes.iter().zip(paths).any(|(n, p)| n.path != *p)
    });
    if changed {
        clear();
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class = w!("BuddyHeldFileLabel");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance.into(),
            lpszClassName: class,
            ..Default::default()
        });
        for path in paths {
            if let Ok(window) = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOPMOST,
                class,
                w!("Held file"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                Some(owner),
                None,
                Some(instance.into()),
                None,
            ) {
                let _ = SetLayeredWindowAttributes(window, COLORREF(0), 255, LWA_COLORKEY);
                let label = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned();
                HALO.with(|h| {
                    h.borrow_mut().nodes.push(Node {
                        window,
                        path: path.clone(),
                        label,
                        kind: kind(path),
                    })
                });
            }
        }
    }
    let mut pet = RECT::default();
    let _ = GetWindowRect(owner, &mut pet);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(
        MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
        &mut info,
    )
    .as_bool()
    {
        return;
    }
    let rects = positions(
        pet,
        info.rcWork,
        paths.len(),
        GetDpiForWindow(owner).max(96) as i32,
    );
    let windows = HALO.with(|h| {
        let mut h = h.borrow_mut();
        h.palette = Some(palette);
        h.nodes.iter().map(|n| n.window).collect::<Vec<_>>()
    });
    for (window, rect) in windows.into_iter().zip(rects) {
        let _ = SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = InvalidateRect(Some(window), None, false);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_file_cluster_without_desktop_windows() {
        unsafe {
            let (width, height) = (760, 360);
            let dc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: 40,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, bitmap.into());
            let work = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            let brush = CreateSolidBrush(COLORREF(0x00201e1c));
            FillRect(dc, &work, brush);
            let _ = DeleteObject(brush.into());
            let pet = RECT {
                left: 316,
                top: 112,
                right: 444,
                bottom: 248,
            };
            let appearance = crate::pet_art::Appearance::Calico;
            let labels = [
                ("Research", Kind::Folder),
                ("Assignment.pdf", Kind::Pdf),
                ("Notes.txt", Kind::Text),
                ("Budget.xlsx", Kind::Sheet),
                ("Diagram.png", Kind::Image),
                ("Solution.py", Kind::Code),
            ];
            for (rect, (label, kind)) in positions(pet, work, labels.len(), 96)
                .into_iter()
                .zip(labels)
            {
                draw(dc, rect, label, kind, appearance.palette(), 96);
            }
            let saved = SaveDC(dc);
            let _ = SetViewportOrgEx(dc, pet.left, pet.top, None);
            crate::pet_art::draw_pose(
                dc,
                appearance,
                false,
                96,
                crate::pet_art::AnimationSet::reduced_motion_pose(
                    crate::pet_art::AnimationPhase::Held,
                ),
                Some((0, 0)),
                true,
                None,
            );
            let _ = RestoreDC(dc, saved);
            if let Some(path) = std::env::var_os("BUDDY_FILE_CLUSTER_PREVIEW") {
                let pixels =
                    std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize);
                let rgb: Vec<u8> = pixels
                    .chunks_exact(4)
                    .flat_map(|p| [p[2], p[1], p[0]])
                    .collect();
                let mut out = Vec::new();
                {
                    let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
                    encoder.set_color(png::ColorType::Rgb);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&rgb)
                        .unwrap();
                }
                std::fs::write(path, out).unwrap();
            }
            SelectObject(dc, old);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(dc);
        }
    }
    #[test]
    fn file_types_and_all_labels_fit_normal_desktop_edges() {
        assert_eq!(kind(Path::new("report.PDF")), Kind::Pdf);
        assert_eq!(kind(Path::new("budget.xlsx")), Kind::Sheet);
        assert_eq!(kind(Path::new("notes.txt")), Kind::Text);
        let work = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        for scale in [96, 144, 192] {
            for (x, y) in [(0, 0), (1792, 900), (900, 450)] {
                let pet = RECT {
                    left: x,
                    top: y,
                    right: x + 128,
                    bottom: y + 136,
                };
                let rects = positions(pet, work, 20, scale);
                assert_eq!(rects.len(), 20);
                for r in &rects {
                    assert!(
                        r.left >= work.left
                            && r.right <= work.right
                            && r.top >= work.top
                            && r.bottom <= work.bottom
                    );
                }
                for (i, a) in rects.iter().enumerate() {
                    for b in &rects[i + 1..] {
                        assert!(
                            a.right <= b.left
                                || b.right <= a.left
                                || a.bottom <= b.top
                                || b.bottom <= a.top
                        );
                    }
                }
            }
        }
    }
}

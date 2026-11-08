//! Pet-colored native menus. Windows retains selection, keyboard navigation and dismissal.
use crate::pet_art::Palette;
use windows::{
    core::{w, PWSTR, BOOL},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{
            Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW, HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent},
            Controls::*,
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::*,
        },
    },
};

struct MenuLabel {
    text: Vec<u16>,
    palette: Palette,
    font: HFONT,
    scale: i32,
    width: u32,
    separator: bool,
    menu: HMENU,
    index: u32,
    original_type: MENU_ITEM_TYPE,
    original_data: usize,
}

pub(crate) struct MenuTheme {
    // Boxes keep item-data pointers stable while the vector grows.
    labels: Vec<Box<MenuLabel>>,
    background: HBRUSH,
    font: HFONT,
    menus: Vec<(HMENU, HBRUSH)>,
}

impl Drop for MenuTheme {
    fn drop(&mut self) {
        unsafe {
            // System edit menus can outlive the callback that displayed them.
            for label in &self.labels {
                let mut item = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_DATA,
                    ..Default::default()
                };
                if GetMenuItemInfoW(label.menu, label.index, true, &mut item).is_ok()
                    && item.dwItemData == label.as_ref() as *const MenuLabel as usize
                {
                    item.fMask = MIIM_FTYPE | MIIM_DATA;
                    item.fType = label.original_type;
                    item.dwItemData = label.original_data;
                    let _ = SetMenuItemInfoW(label.menu, label.index, true, &item);
                }
            }
            for (menu, background) in &self.menus {
                let mut info = MENUINFO {
                    cbSize: std::mem::size_of::<MENUINFO>() as u32,
                    fMask: MIM_BACKGROUND,
                    ..Default::default()
                };
                if GetMenuInfo(*menu, &mut info).is_ok() && info.hbrBack == self.background {
                    info.hbrBack = *background;
                    let _ = SetMenuInfo(*menu, &info);
                }
            }
            let _ = DeleteObject(self.font.into());
            let _ = DeleteObject(self.background.into());
        }
    }
}

impl MenuTheme {
    unsafe fn new(palette: Palette, dpi: u32) -> Self {
        Self {
            labels: Vec::new(),
            menus: Vec::new(),
            background: CreateSolidBrush(palette.surface),
            font: CreateFontW(
                -14 * dpi as i32 / 96,
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
                w!("Trebuchet MS"),
            ),
        }
    }

    pub(crate) unsafe fn apply(&mut self, menu: HMENU, owner: HWND, palette: Palette, dpi: u32) {
        let scale = dpi as i32;
        let info = MENUINFO {
            cbSize: std::mem::size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND,
            hbrBack: self.background,
            ..Default::default()
        };
        if !self.menus.iter().any(|(existing, _)| *existing == menu) {
            let mut original = MENUINFO {
                cbSize: std::mem::size_of::<MENUINFO>() as u32,
                fMask: MIM_BACKGROUND,
                ..Default::default()
            };
            if GetMenuInfo(menu, &mut original).is_ok() {
                self.menus.push((menu, original.hbrBack));
            }
        }
        let _ = SetMenuInfo(menu, &info);
        let dc = GetDC(Some(owner));
        let old_font = SelectObject(dc, self.font.into());
        let monitor = MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut monitor_info);
        let available = (monitor_info.rcWork.right - monitor_info.rcWork.left - 24).max(120);
        for index in 0..GetMenuItemCount(Some(menu)).max(0) as u32 {
            let mut text = vec![0u16; 512];
            let mut item = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_STRING | MIIM_SUBMENU | MIIM_DATA,
                dwTypeData: PWSTR(text.as_mut_ptr()),
                cch: text.len() as u32,
                ..Default::default()
            };
            if GetMenuItemInfoW(menu, index, true, &mut item).is_err() {
                continue;
            }
            text.truncate(item.cch as usize);
            let separator = item.fType.contains(MFT_SEPARATOR);
            // Windows may initialize the same submenu more than once.
            if self
                .labels
                .iter()
                .any(|label| label.as_ref() as *const MenuLabel as usize == item.dwItemData)
            {
                continue;
            }
            let mut size = SIZE::default();
            let _ = GetTextExtentPoint32W(dc, &text, &mut size);
            let label = Box::new(MenuLabel {
                text,
                palette,
                font: self.font,
                scale,
                width: (size.cx + 64 * scale / 96)
                    .max(160 * scale / 96)
                    .min(available) as u32,
                separator,
                menu,
                index,
                original_type: item.fType,
                original_data: item.dwItemData,
            });
            let data = label.as_ref() as *const MenuLabel as usize;
            self.labels.push(label);
            let themed = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_DATA,
                fType: item.fType | MFT_OWNERDRAW,
                dwItemData: data,
                ..Default::default()
            };
            let _ = SetMenuItemInfoW(menu, index, true, &themed);
            if !item.hSubMenu.0.is_null() {
                self.apply(item.hSubMenu, owner, palette, dpi);
            }
        }
        SelectObject(dc, old_font);
        ReleaseDC(Some(owner), dc);
    }
}

/// Takes ownership of the menu and all of its submenus.
pub unsafe fn track(menu: HMENU, owner: HWND, x: i32, y: i32, palette: Palette) -> u32 {
    let _front = MenuFront::new(owner);
    let theme = prepare(menu, owner, palette);
    let _ = SetForegroundWindow(owner);
    let action = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        x,
        y,
        None,
        owner,
        None,
    )
    .0;
    let _ = DestroyMenu(menu);
    drop(theme);
    let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
    action as u32
}

// Applies to newly opened submenu windows as well as the first popup.
unsafe extern "system" fn raise_popup(
    _: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    _: i32,
    _: i32,
    thread: u32,
    _: u32,
) {
    if event != EVENT_SYSTEM_MENUPOPUPSTART || window.0.is_null() {
        return;
    }
    let _ = raise_menu_window(window, LPARAM(0));
    // MENUPOPUPSTART may identify the activating control rather than the popup.
    let _ = EnumThreadWindows(thread, Some(raise_menu_window), LPARAM(0));
}
unsafe extern "system" fn raise_menu_window(window: HWND, _: LPARAM) -> BOOL {
    let mut class = [0u16; 32];
    let count = GetClassNameW(window, &mut class);
    if String::from_utf16_lossy(&class[..count.max(0) as usize]) == "#32768" {
        let _ = SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        );
    }
    BOOL(1)
}
struct MenuFront {
    owner: HWND,
    style: isize,
    hook: HWINEVENTHOOK,
    previous_menu: bool,
}
impl MenuFront {
    unsafe fn new(owner: HWND) -> Self {
        let previous_menu = crate::file_halo::menu_open(true);
        let style = GetWindowLongPtrW(owner, GWL_EXSTYLE);
        // The pet normally declines activation; an explicit context menu needs
        // a foreground owner so Windows gives it normal dismissal and z-order.
        if style & WS_EX_NOACTIVATE.0 as isize != 0 {
            SetWindowLongPtrW(owner, GWL_EXSTYLE, style & !(WS_EX_NOACTIVATE.0 as isize));
        }
        let _ = SetWindowPos(
            owner,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        let thread = GetWindowThreadProcessId(owner, None);
        let hook = SetWinEventHook(
            EVENT_SYSTEM_MENUPOPUPSTART,
            EVENT_SYSTEM_MENUPOPUPSTART,
            None,
            Some(raise_popup),
            std::process::id(),
            thread,
            WINEVENT_OUTOFCONTEXT,
        );
        Self {
            owner,
            style,
            hook,
            previous_menu,
        }
    }
}
impl Drop for MenuFront {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.0.is_null() {
                let _ = UnhookWinEvent(self.hook);
            }
            if IsWindow(Some(self.owner)).as_bool() {
                SetWindowLongPtrW(self.owner, GWL_EXSTYLE, self.style);
                if self.style & WS_EX_TOPMOST.0 as isize == 0 {
                    let _ = SetWindowPos(
                        self.owner,
                        Some(HWND_NOTOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
            crate::file_halo::menu_open(self.previous_menu);
        }
    }
}

/// Keeps drawing resources alive for a menu owned and destroyed by Windows.
pub(crate) unsafe fn prepare(menu: HMENU, owner: HWND, palette: Palette) -> Option<MenuTheme> {
    let dpi = GetDpiForWindow(owner).max(96);
    let mut contrast = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    let _ = SystemParametersInfoW(
        SPI_GETHIGHCONTRAST,
        contrast.cbSize,
        Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    );
    let mut theme =
        (!contrast.dwFlags.contains(HCF_HIGHCONTRASTON)).then(|| MenuTheme::new(palette, dpi));
    if let Some(theme) = &mut theme {
        theme.apply(menu, owner, palette, dpi);
    }
    theme
}

pub unsafe fn handle_message(msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_MENUCHAR => {
            let character = char::from_u32((wp.0 & 0xffff) as u32)?;
            let menu = HMENU(lp.0 as _);
            let mut matches = Vec::new();
            let mut current = None;
            for index in 0..GetMenuItemCount(Some(menu)).max(0) as u32 {
                let mut item = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_DATA | MIIM_FTYPE | MIIM_STATE,
                    ..Default::default()
                };
                if GetMenuItemInfoW(menu, index, true, &mut item).is_err()
                    || !item.fType.contains(MFT_OWNERDRAW)
                    || item.dwItemData == 0
                {
                    continue;
                }
                if item.fState.0 & MFS_HILITE.0 != 0 {
                    current = Some(index);
                }
                if item.fState.0 & MFS_DISABLED.0 == 0 {
                    let label = &*(item.dwItemData as *const MenuLabel);
                    if mnemonic_matches(&label.text, character) {
                        matches.push(index);
                    }
                }
            }
            let first = *matches.first()?;
            let index = current
                .and_then(|current| matches.iter().copied().find(|index| *index > current))
                .unwrap_or(first);
            let action = if matches.len() == 1 {
                MNC_EXECUTE
            } else {
                MNC_SELECT
            };
            Some(LRESULT((index | (action << 16)) as isize))
        }
        WM_MEASUREITEM => {
            let measure = (lp.0 as *mut MEASUREITEMSTRUCT).as_mut()?;
            if measure.CtlType != ODT_MENU || measure.itemData == 0 {
                return None;
            }
            let label = &*(measure.itemData as *const MenuLabel);
            measure.itemWidth = label.width;
            measure.itemHeight = ((if label.separator { 9 } else { 32 }) * label.scale / 96) as u32;
            Some(LRESULT(1))
        }
        WM_DRAWITEM => {
            let draw = (lp.0 as *const DRAWITEMSTRUCT).as_ref()?;
            if draw.CtlType != ODT_MENU || draw.itemData == 0 {
                return None;
            }
            let label = &*(draw.itemData as *const MenuLabel);
            let s = |n| n * label.scale / 96;
            let selected = draw.itemState.0 & ODS_SELECTED.0 != 0;
            let disabled = draw.itemState.0 & (ODS_DISABLED.0 | ODS_GRAYED.0) != 0;
            let dc = draw.hDC;
            let saved = SaveDC(dc);
            let background = CreateSolidBrush(if selected {
                label.palette.selection
            } else {
                label.palette.surface
            });
            let _ = FillRect(dc, &draw.rcItem, background);
            let _ = DeleteObject(background.into());
            let mut rect = draw.rcItem;
            if label.separator {
                let pen = CreatePen(PS_SOLID, 1, label.palette.outline);
                SelectObject(dc, pen.into());
                let middle = (rect.top + rect.bottom) / 2;
                let _ = MoveToEx(dc, rect.left + s(14), middle, None);
                let _ = LineTo(dc, rect.right - s(14), middle);
                let _ = RestoreDC(dc, saved);
                let _ = DeleteObject(pen.into());
                return Some(LRESULT(1));
            }
            SelectObject(dc, label.font.into());
            SetBkMode(dc, TRANSPARENT);
            let ink = if disabled && draw.itemID != 0 {
                label.palette.outline
            } else if selected || disabled {
                label.palette.accent
            } else {
                label.palette.ink
            };
            SetTextColor(dc, ink);
            rect.left += s(28);
            rect.right -= s(28);
            let mut text = label.text.clone();
            DrawTextW(
                dc,
                &mut text,
                &mut rect,
                DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            if draw.itemState.0 & ODS_CHECKED.0 != 0 {
                let mut check: Vec<_> = "✓".encode_utf16().collect();
                let mut bounds = RECT {
                    left: draw.rcItem.left + s(7),
                    right: draw.rcItem.left + s(25),
                    ..draw.rcItem
                };
                DrawTextW(
                    dc,
                    &mut check,
                    &mut bounds,
                    DT_SINGLELINE | DT_VCENTER | DT_CENTER,
                );
            }
            let _ = RestoreDC(dc, saved);
            Some(LRESULT(1))
        }
        _ => None,
    }
}

fn mnemonic_matches(text: &[u16], character: char) -> bool {
    let text = String::from_utf16_lossy(text);
    let mut characters = text.chars();
    while let Some(next) = characters.next() {
        if next == '&' {
            if let Some(mnemonic) = characters.next() {
                if mnemonic != '&' && mnemonic.to_lowercase().eq(character.to_lowercase()) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn menu_front_guard_restores_hidden_owner_styles_without_showing_it() {
        unsafe {
            let owner=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,w!("STATIC"),w!("Hidden menu owner fixture"),WS_POPUP,0,0,1,1,None,None,None,None).unwrap();
            let original=GetWindowLongPtrW(owner,GWL_EXSTYLE);
            {let _guard=MenuFront::new(owner);
                let active=GetWindowLongPtrW(owner,GWL_EXSTYLE);
                assert_eq!(active & WS_EX_NOACTIVATE.0 as isize,0);
                // Hidden-window z-order can be deferred by Windows. This
                // fixture verifies activation-style lifetime without showing
                // a menu; visible stacking remains a device acceptance check.
                assert!(!IsWindowVisible(owner).as_bool());
            }
            assert_eq!(GetWindowLongPtrW(owner,GWL_EXSTYLE),original);
            assert!(!IsWindowVisible(owner).as_bool());
            DestroyWindow(owner).unwrap();
        }
    }
    use crate::pet_art::Appearance;

    #[test]
    fn native_menu_mnemonics_and_original_style_are_preserved() {
        unsafe {
            let menu = CreatePopupMenu().unwrap();
            AppendMenuW(menu, MF_STRING, 1, w!("&Copy")).unwrap();
            AppendMenuW(menu, MF_STRING, 2, w!("Cu&t")).unwrap();
            let mut theme = MenuTheme::new(Appearance::Calico.palette(), 96);
            theme.apply(menu, HWND::default(), Appearance::Calico.palette(), 96);
            let key =
                handle_message(WM_MENUCHAR, WPARAM('c' as usize), LPARAM(menu.0 as isize)).unwrap();
            assert_eq!(key.0 as u32, MNC_EXECUTE << 16);
            assert!(!mnemonic_matches(
                &"Cats && dogs".encode_utf16().collect::<Vec<_>>(),
                'd'
            ));
            drop(theme);
            let mut item = MENUITEMINFOW {
                cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_DATA,
                ..Default::default()
            };
            GetMenuItemInfoW(menu, 0, true, &mut item).unwrap();
            assert!(!item.fType.contains(MFT_OWNERDRAW));
            assert_eq!(item.dwItemData, 0);
            DestroyMenu(menu).unwrap();
        }
    }

    #[test]
    fn menu_labels_submenus_and_dpi_survive_theming() {
        unsafe {
            for dpi in [96, 144, 192] {
                let menu = CreatePopupMenu().unwrap();
                let child = CreatePopupMenu().unwrap();
                AppendMenuW(child, MF_STRING, 42, w!("Mark complete")).unwrap();
                AppendMenuW(menu, MF_POPUP, child.0 as usize, w!("Café 猫")).unwrap();
                let mut theme = MenuTheme::new(Appearance::Calico.palette(), dpi);
                theme.apply(menu, HWND::default(), Appearance::Calico.palette(), dpi);
                let mut text = [0u16; 64];
                let mut item = MENUITEMINFOW {
                    cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                    fMask: MIIM_STRING | MIIM_SUBMENU | MIIM_FTYPE | MIIM_DATA,
                    dwTypeData: PWSTR(text.as_mut_ptr()),
                    cch: text.len() as u32,
                    ..Default::default()
                };
                GetMenuItemInfoW(menu, 0, true, &mut item).unwrap();
                assert_eq!(
                    String::from_utf16_lossy(&text[..item.cch as usize]),
                    "Café 猫"
                );
                assert_eq!(item.hSubMenu, child);
                assert!(item.fType.contains(MFT_OWNERDRAW));
                let mut measure = MEASUREITEMSTRUCT {
                    CtlType: ODT_MENU,
                    itemData: item.dwItemData,
                    ..Default::default()
                };
                assert_eq!(
                    handle_message(
                        WM_MEASUREITEM,
                        WPARAM(0),
                        LPARAM((&mut measure as *mut MEASUREITEMSTRUCT) as isize)
                    ),
                    Some(LRESULT(1))
                );
                assert_eq!(measure.itemHeight, 32 * dpi / 96);
                assert!(measure.itemWidth >= 160 * dpi / 96);
                assert_eq!(theme.labels.len(), 2);
                DestroyMenu(menu).unwrap();
            }
        }
    }

    #[test]
    fn renders_menu_states_without_opening_a_window() {
        unsafe {
            const WIDTH: i32 = 960;
            const HEIGHT: i32 = 240;
            let dc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: 40,
                    biWidth: WIDTH,
                    biHeight: -HEIGHT,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let previous = SelectObject(dc, bitmap.into());
            for (column, (pet, name)) in [
                (Appearance::Calico, w!("Calico · tools")),
                (Appearance::Dog, w!("Shiba Inu · tools")),
                (Appearance::Panda, w!("Panda · tools")),
            ]
            .into_iter()
            .enumerate()
            {
                let palette = pet.palette();
                let menu = CreatePopupMenu().unwrap();
                let child = CreatePopupMenu().unwrap();
                AppendMenuW(child, MF_STRING, 8, w!("Mark complete")).unwrap();
                AppendMenuW(menu, MF_STRING | MF_DISABLED, 0, name).unwrap();
                AppendMenuW(menu, MF_STRING, 1, w!("Open tools — Café 猫")).unwrap();
                AppendMenuW(menu, MF_STRING, 2, w!("Toggle toolbar")).unwrap();
                AppendMenuW(menu, MF_SEPARATOR, 0, None).unwrap();
                AppendMenuW(menu, MF_POPUP, child.0 as usize, w!("Choose pet")).unwrap();
                AppendMenuW(menu, MF_STRING | MF_CHECKED, 3, w!("Current pet")).unwrap();
                AppendMenuW(menu, MF_STRING | MF_DISABLED, 4, w!("Copy held files")).unwrap();
                let mut theme = MenuTheme::new(palette, 96);
                theme.apply(menu, HWND::default(), palette, 96);
                let background = CreateSolidBrush(palette.surface);
                let left = column as i32 * 320;
                let _ = FillRect(
                    dc,
                    &RECT {
                        left,
                        top: 0,
                        right: left + 320,
                        bottom: HEIGHT,
                    },
                    background,
                );
                assert!(DeleteObject(background.into()).as_bool());
                let mut top = 0;
                for index in 0..7 {
                    let mut item = MENUITEMINFOW {
                        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
                        fMask: MIIM_DATA,
                        ..Default::default()
                    };
                    GetMenuItemInfoW(menu, index, true, &mut item).unwrap();
                    let label = &*(item.dwItemData as *const MenuLabel);
                    let height = if label.separator { 9 } else { 32 };
                    let state = match index {
                        0 | 6 => ODS_DISABLED,
                        2 => ODS_SELECTED,
                        5 => ODS_CHECKED,
                        _ => ODS_FLAGS(0),
                    };
                    let draw = DRAWITEMSTRUCT {
                        CtlType: ODT_MENU,
                        hDC: dc,
                        itemState: state,
                        itemID: GetMenuItemID(menu, index as i32),
                        rcItem: RECT {
                            left,
                            top,
                            right: left + 320,
                            bottom: top + height,
                        },
                        itemData: item.dwItemData,
                        ..Default::default()
                    };
                    assert_eq!(
                        handle_message(
                            WM_DRAWITEM,
                            WPARAM(0),
                            LPARAM((&draw as *const DRAWITEMSTRUCT) as isize)
                        ),
                        Some(LRESULT(1))
                    );
                    let expected = if index == 2 {
                        palette.selection
                    } else {
                        palette.surface
                    };
                    assert_eq!(GetPixel(dc, left + 2, top + 1), expected);
                    top += height;
                }
                DestroyMenu(menu).unwrap();
            }
            let _ = GdiFlush();
            if let Some(path) = std::env::var_os("BUDDY_MENU_PREVIEW") {
                let pixels =
                    std::slice::from_raw_parts(bits.cast::<u8>(), (WIDTH * HEIGHT * 4) as usize);
                let mut bmp = Vec::new();
                bmp.extend(b"BM");
                bmp.extend(((54 + pixels.len()) as u32).to_le_bytes());
                bmp.extend(0u32.to_le_bytes());
                bmp.extend(54u32.to_le_bytes());
                bmp.extend(40u32.to_le_bytes());
                bmp.extend(WIDTH.to_le_bytes());
                bmp.extend((-HEIGHT).to_le_bytes());
                bmp.extend(1u16.to_le_bytes());
                bmp.extend(32u16.to_le_bytes());
                bmp.extend([0u8; 24]);
                bmp.extend(pixels);
                std::fs::write(path, bmp).unwrap();
            }
            SelectObject(dc, previous);
            assert!(DeleteObject(bitmap.into()).as_bool());
            let _ = DeleteDC(dc);
        }
    }
}

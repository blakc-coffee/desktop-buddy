//! Explicit text/link references. Only app-created cache paths enter this registry.
use super::*;
use std::{collections::HashMap, sync::OnceLock};
#[derive(Clone, Serialize)]
pub struct Reference {
    pub text: String,
    pub link: bool,
    pub label: String,
}
fn registry() -> &'static Mutex<HashMap<PathBuf, Reference>> {
    static ITEMS: OnceLock<Mutex<HashMap<PathBuf, Reference>>> = OnceLock::new();
    ITEMS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn get(path: &Path) -> Option<Reference> {
    registry().lock().ok()?.get(path).cloned()
}
pub fn describe(path: &Path) -> Option<(String, bool)> {
    let items = registry().lock().ok()?;
    let item = items.get(path)?;
    Some((item.label.clone(), item.link))
}
pub fn clear() {
    if let Ok(mut items) = registry().lock() {
        items.clear();
    }
}
pub fn forget(path: &Path) {
    if let Ok(mut items) = registry().lock() {
        items.remove(path);
    }
}
pub fn materialize(text: String, link: bool, root: &Path) -> Result<Vec<PathBuf>, String> {
    if text.trim().is_empty() || text.len() > 32 * 1024 || text.contains('\0') {
        return Err("Hold text or a link up to 32 KiB.".into());
    }
    if link {
        let url = tauri::Url::parse(text.trim()).map_err(err)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err("Use an HTTP or HTTPS link without credentials.".into());
        }
    }
    if registry().lock().map_err(err)?.len() >= 64 {
        return Err("Clear held references before adding more text.".into());
    }
    let label = if link {
        let url = tauri::Url::parse(text.trim()).map_err(err)?;
        format!("Link · {}{}", url.host_str().unwrap_or("link"), url.path())
            .chars()
            .take(70)
            .collect()
    } else {
        format!(
            "Text · {}",
            text.lines()
                .find(|line| !line.trim().is_empty())
                .unwrap_or("Text")
                .trim()
        )
        .chars()
        .take(70)
        .collect()
    };
    let paths = crate::drop_data::materialize(
        crate::drop_data::DropData::Files(vec![(
            if link { "link.url" } else { "text.txt" }.into(),
            if link {
                format!(
                    "[InternetShortcut]\r\nURL={}\r\n",
                    tauri::Url::parse(text.trim()).map_err(err)?.as_str()
                )
                .into_bytes()
            } else {
                text.as_bytes().to_vec()
            },
        )]),
        root,
    )?;
    let mut items = registry().lock().map_err(err)?;
    if items.len() >= 64 {
        return Err("Clear held references before adding more text.".into());
    }
    items.insert(paths[0].clone(), Reference { text, link, label });
    Ok(paths)
}
pub unsafe fn add_transfer_formats(
    data: &windows::Win32::System::Com::IDataObject,
    reference: &Reference,
) -> Result<(), String> {
    use windows::Win32::{
        Foundation::*,
        System::{Com::*, DataExchange::*, Memory::*, Ole::*},
    };
    let bytes: Vec<u16> = reference.text.encode_utf16().chain(Some(0)).collect();
    let mut formats = vec![CF_UNICODETEXT.0];
    if reference.link {
        let name: Vec<u16> = "UniformResourceLocatorW"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        formats.push(RegisterClipboardFormatW(windows::core::PCWSTR(name.as_ptr())) as u16);
    }
    for format in formats {
        let global = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len() * 2).map_err(err)?;
        let pointer = GlobalLock(global);
        if pointer.is_null() {
            let _ = GlobalFree(Some(global));
            return Err("Could not prepare text transfer.".into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u16>(), bytes.len());
        let _ = GlobalUnlock(global);
        let request = FORMATETC {
            cfFormat: format,
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
            ..Default::default()
        };
        let mut medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: global },
            ..Default::default()
        };
        if let Err(error) = data.SetData(&request, &medium, true) {
            ReleaseStgMedium(&mut medium);
            return Err(err(error));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_transfer_keeps_unicode_text_and_url_formats() {
        unsafe {
            use windows::Win32::{
                System::{Com::*, Ole::*},
                UI::Shell::*,
            };
            OleInitialize(None).unwrap();
            let root = std::env::temp_dir().join(format!("buddy-text-transfer-{}", new_id()));
            let paths = materialize("https://example.com/猫?q=1".into(), true, &root).unwrap();
            let wide: Vec<u16> = paths[0]
                .to_string_lossy()
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let id = ILCreateFromPathW(windows::core::PCWSTR(wide.as_ptr()));
            assert!(!id.is_null());
            let array = SHCreateShellItemArrayFromIDLists(&[id as *const _]).unwrap();
            CoTaskMemFree(Some(id as _));
            let data: IDataObject = array
                .BindToHandler(None::<&IBindCtx>, &BHID_DataObject)
                .unwrap();
            add_transfer_formats(&data, &get(&paths[0]).unwrap()).unwrap();
            let request = FORMATETC {
                cfFormat: CF_UNICODETEXT.0,
                dwAspect: DVASPECT_CONTENT.0,
                lindex: -1,
                tymed: TYMED_HGLOBAL.0 as u32,
                ..Default::default()
            };
            let mut medium = data.GetData(&request).unwrap();
            let pointer =
                windows::Win32::System::Memory::GlobalLock(medium.u.hGlobal).cast::<u16>();
            let expected: Vec<u16> = "https://example.com/猫?q=1".encode_utf16().collect();
            assert_eq!(
                std::slice::from_raw_parts(pointer, expected.len()),
                expected
            );
            let _ = windows::Win32::System::Memory::GlobalUnlock(medium.u.hGlobal);
            ReleaseStgMedium(&mut medium);
            drop(data);
            drop(array);
            forget(&paths[0]);
            std::fs::remove_file(&paths[0]).unwrap();
            std::fs::remove_dir(paths[0].parent().unwrap()).unwrap();
            std::fs::remove_dir(&root).unwrap();
            OleUninitialize();
        }
    }
}

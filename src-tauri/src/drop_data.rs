//! Bounded imports for editor resources, virtual attachments and browser images.
use super::{err, new_id};
use base64::Engine;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Networking::WinHttp::*,
        System::{Com::*, DataExchange::*, Memory::*, Ole::*},
        UI::Shell::*,
    },
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
};

const MAX_FILE: usize = 16 * 1024 * 1024;
const MAX_BATCH: usize = 32 * 1024 * 1024;
const MAX_METADATA: usize = 48 * 1024 * 1024;
const MAX_CACHE: u64 = 128 * 1024 * 1024;
const CUSTOM: &str = "Chromium Web Custom MIME Data Format";
const TEXT_FORMATS: [&str; 7] = [
    "CodeFiles",
    "CodeEditors",
    "ResourceURLs",
    "application/vnd.code.uri-list",
    "text/uri-list",
    "DownloadURL",
    "UniformResourceLocatorW",
];

pub enum DropData {
    Paths(Vec<PathBuf>),
    Files(Vec<(String, Vec<u8>)>),
    Download { url: tauri::Url, name: String },
}

fn format(name: &str) -> FORMATETC {
    let name: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
    FORMATETC {
        cfFormat: unsafe { RegisterClipboardFormatW(PCWSTR(name.as_ptr())) } as u16,
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
        ..Default::default()
    }
}

struct Medium(STGMEDIUM);
impl Drop for Medium {
    fn drop(&mut self) {
        unsafe {
            ReleaseStgMedium(&mut self.0);
        }
    }
}

unsafe fn bytes(medium: &Medium, limit: usize) -> Result<Vec<u8>, String> {
    if medium.0.tymed != TYMED_HGLOBAL.0 as u32 {
        return Err("The source offered an unsupported storage format.".into());
    }
    let global = medium.0.u.hGlobal;
    let size = GlobalSize(global);
    if size == 0 || size > limit {
        return Err("The dragged data exceeds Buddy’s import limit.".into());
    }
    let pointer = GlobalLock(global);
    if pointer.is_null() {
        return Err("Could not read the dragged data.".into());
    }
    let result = std::slice::from_raw_parts(pointer.cast::<u8>(), size).to_vec();
    let _ = GlobalUnlock(global);
    Ok(result)
}

pub unsafe fn supported(data: &IDataObject) -> bool {
    [
        format("FileGroupDescriptorW"),
        format("PNG"),
        format("image/png"),
        format(CUSTOM),
    ]
    .into_iter()
    .chain(TEXT_FORMATS.into_iter().map(format))
    .chain(
        [CF_UNICODETEXT.0, CF_DIB.0, CF_DIBV5.0]
            .into_iter()
            .map(|cf_format| FORMATETC {
                cfFormat: cf_format,
                ..format("text/plain")
            }),
    )
    .any(|format| data.QueryGetData(&format).is_ok())
}

unsafe fn offered(data: &IDataObject, request: &FORMATETC) -> Option<Medium> {
    if data.QueryGetData(request).is_err() {
        return None;
    }
    data.GetData(request).ok().map(Medium)
}

pub unsafe fn read(data: &IDataObject) -> Result<DropData, String> {
    if let Some(medium) = offered(data, &format("FileGroupDescriptorW")) {
        return virtual_files(data, bytes(&medium, 64 * 1024)?);
    }
    for name in ["PNG", "image/png"] {
        if let Some(medium) = offered(data, &format(name)) {
            let content = bytes(&medium, MAX_FILE)?;
            if !content.starts_with(b"\x89PNG\r\n\x1a\n") {
                return Err("The image payload is not a PNG file.".into());
            }
            return Ok(DropData::Files(vec![("image.png".into(), content)]));
        }
    }
    for cf_format in [CF_DIBV5.0, CF_DIB.0] {
        let request = FORMATETC {
            cfFormat: cf_format,
            ..format("image/bmp")
        };
        if let Some(medium) = offered(data, &request) {
            let dib = bytes(&medium, MAX_FILE - 14)?;
            return Ok(DropData::Files(vec![(
                "image.bmp".into(),
                bitmap_file(&dib)?,
            )]));
        }
    }
    let mut payloads = Vec::new();
    if let Some(medium) = offered(data, &format(CUSTOM)) {
        let value = bytes(&medium, MAX_METADATA)?;
        payloads.extend(custom_payloads(&value)?);
    }
    for name in TEXT_FORMATS {
        if let Some(medium) = offered(data, &format(name)) {
            let value = bytes(&medium, MAX_METADATA)?;
            payloads.push((
                name.to_string(),
                decode_text(&value, name == "UniformResourceLocatorW")?,
            ));
        }
    }
    let text = FORMATETC {
        cfFormat: CF_UNICODETEXT.0,
        ..format("text/plain")
    };
    if let Some(medium) = offered(data, &text) {
        payloads.push((
            "text/uri-list".into(),
            decode_text(&bytes(&medium, MAX_METADATA)?, true)?,
        ));
    }
    let mut last_error = None;
    for name in TEXT_FORMATS {
        for (_, value) in payloads
            .iter()
            .filter(|(kind, _)| kind.eq_ignore_ascii_case(name))
        {
            match parse_text(name, value) {
                Ok(value) => {
                    if let DropData::Paths(paths) = &value {
                        if let Err(error) = super::pet::validated(paths.clone()) {
                            last_error = Some(error);
                            continue;
                        }
                    }
                    return Ok(value);
                }
                Err(error) => last_error = Some(error),
            }
        }
    }
    Err(last_error.unwrap_or_else(|| "This drag contains no usable file data. Try the attachment’s download or drag-file action.".into()))
}

fn decode_text(bytes: &[u8], wide: bool) -> Result<String, String> {
    let wide = wide || (bytes.len() > 3 && bytes[1] == 0 && bytes[3] == 0);
    if wide {
        if bytes.len() % 2 != 0 {
            return Err("Invalid Unicode drag data.".into());
        }
        let units: Vec<_> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|unit| *unit != 0)
            .collect();
        String::from_utf16(&units).map_err(err)
    } else {
        String::from_utf8(
            bytes
                .iter()
                .copied()
                .take_while(|byte| *byte != 0)
                .collect(),
        )
        .map_err(err)
    }
}

fn take_u32(data: &mut &[u8]) -> Result<u32, String> {
    let word = data.get(..4).ok_or("Truncated editor drag data.")?;
    let value = u32::from_le_bytes(word.try_into().map_err(err)?);
    *data = &data[4..];
    Ok(value)
}

fn bitmap_file(dib: &[u8]) -> Result<Vec<u8>, String> {
    let word = |offset| -> Result<u32, String> {
        Ok(u32::from_le_bytes(
            dib.get(offset..offset + 4)
                .ok_or("Truncated image header.")?
                .try_into()
                .map_err(err)?,
        ))
    };
    let header = word(0)? as usize;
    let width = word(4)? as i32;
    let height = word(8)? as i32;
    let bits = u16::from_le_bytes(
        dib.get(14..16)
            .ok_or("Truncated image header.")?
            .try_into()
            .map_err(err)?,
    );
    let planes = u16::from_le_bytes(
        dib.get(12..14)
            .ok_or("Truncated image header.")?
            .try_into()
            .map_err(err)?,
    );
    let compression = word(16)?;
    if !matches!(header, 40 | 108 | 124)
        || width <= 0
        || height == 0
        || planes != 1
        || !matches!(bits, 1 | 4 | 8 | 16 | 24 | 32)
        || !matches!(compression, 0 | 3)
    {
        return Err("The source supplied an unsupported bitmap.".into());
    }
    let colors = if bits <= 8 {
        let count = word(32)?;
        if count == 0 {
            1u32 << bits
        } else if count <= 1u32 << bits {
            count
        } else {
            return Err("Invalid image palette.".into());
        }
    } else {
        0
    };
    let offset = header
        + colors as usize * 4
        + if header == 40 && compression == 3 {
            12
        } else {
            0
        };
    let stride = (width as u64 * bits as u64).div_ceil(32) * 4;
    let pixels = stride.saturating_mul(i64::from(height).unsigned_abs());
    if pixels > MAX_FILE as u64
        || offset as u64 + pixels > dib.len() as u64
        || dib.len() > MAX_FILE - 14
    {
        return Err("The image is incomplete or exceeds 16 MiB.".into());
    }
    let mut bmp = Vec::with_capacity(14 + dib.len());
    bmp.extend(b"BM");
    bmp.extend(((14 + dib.len()) as u32).to_le_bytes());
    bmp.extend(0u32.to_le_bytes());
    bmp.extend(((14 + offset) as u32).to_le_bytes());
    bmp.extend(dib);
    Ok(bmp)
}

fn take_string(data: &mut &[u8]) -> Result<String, String> {
    let length = (take_u32(data)? as usize)
        .checked_mul(2)
        .ok_or("Invalid editor text length.")?;
    let padded = length.checked_add(3).ok_or("Invalid editor text length.")? & !3;
    let value = data.get(..length).ok_or("Truncated editor text.")?;
    let value = String::from_utf16(
        &value
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>(),
    )
    .map_err(err)?;
    *data = data.get(padded..).ok_or("Truncated editor text padding.")?;
    Ok(value)
}

fn custom_payloads(bytes: &[u8]) -> Result<Vec<(String, String)>, String> {
    // Chromium Pickle has a four-byte payload size, followed by aligned fields.
    let mut header = bytes;
    let size = take_u32(&mut header)? as usize;
    let mut data = header.get(..size).ok_or("Truncated editor drag payload.")?;
    let count = take_u32(&mut data)?;
    if count > 64 {
        return Err("Too many editor drag formats.".into());
    }
    let mut result = Vec::new();
    for _ in 0..count {
        let name = take_string(&mut data)?;
        let value = take_string(&mut data)?;
        if TEXT_FORMATS
            .iter()
            .any(|kind| name.eq_ignore_ascii_case(kind))
        {
            result.push((name, value));
        }
    }
    Ok(result)
}

fn local_resource(value: &str) -> Result<PathBuf, String> {
    let value = value.trim().trim_matches('"');
    if value.to_ascii_lowercase().starts_with("file:") {
        let mut url = tauri::Url::parse(value).map_err(err)?;
        url.set_fragment(None);
        url.set_query(None);
        return url
            .to_file_path()
            .map_err(|_| "This resource is not a local file.".into());
    }
    if let Ok(url) = tauri::Url::parse(value) {
        let webview_file = url.host_str().is_some_and(|host| {
            host == "file+.vscode-resource.vscode-cdn.net"
                || host == "file+.vscode-resource.vscode-webview.net"
        });
        if (url.scheme() == "vscode-file" && url.host_str() == Some("vscode-app")) || webview_file {
            return tauri::Url::parse(&format!("file://{}", url.path()))
                .map_err(err)?
                .to_file_path()
                .map_err(|_| "This artifact is not a local file.".into());
        }
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err("This resource is not a local file.".into())
    }
}

fn parse_text(kind: &str, text: &str) -> Result<DropData, String> {
    let text = text.trim();
    if kind.eq_ignore_ascii_case("CodeFiles") || kind.eq_ignore_ascii_case("ResourceURLs") {
        let values: Vec<String> = serde_json::from_str(text).map_err(err)?;
        if values.is_empty() || values.len() > 20 {
            return Err("Drop 1–20 files at a time.".into());
        }
        return Ok(DropData::Paths(
            values
                .iter()
                .map(|value| local_resource(value))
                .collect::<Result<_, _>>()?,
        ));
    }
    if kind.eq_ignore_ascii_case("CodeEditors") {
        let values: Vec<serde_json::Value> = serde_json::from_str(text).map_err(err)?;
        if values.is_empty() || values.len() > 20 {
            return Err("Drop 1–20 files at a time.".into());
        }
        let mut paths = Vec::new();
        for value in values {
            let resource = &value["resource"];
            let uri = if let Some(uri) = resource.as_str() {
                uri.to_owned()
            } else if resource["scheme"] == "file"
                && matches!(
                    resource["authority"].as_str(),
                    None | Some("") | Some("localhost")
                )
            {
                paths.push(PathBuf::from(
                    resource["path"]
                        .as_str()
                        .ok_or("Missing editor path.")?
                        .trim_start_matches('/'),
                ));
                continue;
            } else {
                return Err("Save the generated file before dragging it.".into());
            };
            paths.push(local_resource(&uri)?);
        }
        return Ok(DropData::Paths(paths));
    }
    if kind.eq_ignore_ascii_case("DownloadURL") {
        let mut parts = text.splitn(3, ':');
        let _mime = parts.next().ok_or("Missing download type.")?;
        let name = parts.next().ok_or("Missing download name.")?;
        let url = parts.next().ok_or("Missing download address.")?;
        return download(url, name);
    }
    let values: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if values.len() == 1 {
        if values[0].starts_with("data:") {
            return data_url(values[0], None);
        }
        if let Ok(path) = local_resource(values[0]) {
            return Ok(DropData::Paths(vec![path]));
        }
        if let Ok(url) = tauri::Url::parse(values[0]) {
            if matches!(url.scheme(), "http" | "https") {
                return download(values[0], "");
            }
        }
    }
    if values.is_empty() || values.len() > 20 {
        return Err("Drop 1–20 files at a time.".into());
    }
    Ok(DropData::Paths(
        values
            .iter()
            .map(|value| local_resource(value))
            .collect::<Result<_, _>>()?,
    ))
}

fn download(value: &str, name: &str) -> Result<DropData, String> {
    if value.starts_with("data:") {
        return data_url(value, (!name.is_empty()).then_some(name));
    }
    let url = tauri::Url::parse(value).map_err(err)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("The attachment needs a usable HTTP or HTTPS download address.".into());
    }
    let name = if name.is_empty() {
        url.path_segments()
            .and_then(|segments| segments.filter(|part| !part.is_empty()).last())
            .unwrap_or("download")
            .to_owned()
    } else {
        name.to_owned()
    };
    Ok(DropData::Download { url, name })
}

fn data_url(value: &str, name: Option<&str>) -> Result<DropData, String> {
    let (header, body) = value.split_once(',').ok_or("Incomplete embedded image.")?;
    if !header.ends_with(";base64") || body.len() > MAX_FILE * 4 / 3 + 4 {
        return Err("The embedded file is unsupported or exceeds 16 MiB.".into());
    }
    let extension = extension(
        header
            .trim_start_matches("data:")
            .trim_end_matches(";base64"),
    );
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(err)?;
    if bytes.len() > MAX_FILE {
        return Err("The embedded file exceeds 16 MiB.".into());
    }
    Ok(DropData::Files(vec![(
        name.map(str::to_owned)
            .unwrap_or_else(|| format!("image.{extension}")),
        bytes,
    )]))
}

fn extension(mime: &str) -> &'static str {
    match mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        _ => "bin",
    }
}

unsafe fn virtual_files(data: &IDataObject, descriptors: Vec<u8>) -> Result<DropData, String> {
    let mut header = descriptors.as_slice();
    let count = take_u32(&mut header)? as usize;
    let descriptor_size = std::mem::size_of::<FILEDESCRIPTORW>();
    if count == 0 || count > 20 || header.len() < count * descriptor_size {
        return Err("The attachment’s file list is incomplete or too large.".into());
    }
    let mut files = Vec::new();
    let mut total = 0;
    for index in 0..count {
        let descriptor = std::ptr::read_unaligned(
            header
                .as_ptr()
                .add(index * descriptor_size)
                .cast::<FILEDESCRIPTORW>(),
        );
        if descriptor.dwFlags & FD_ATTRIBUTES.0 as u32 != 0
            && descriptor.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0
        {
            return Err("Virtual folders need to be saved before dragging.".into());
        }
        let filename = descriptor.cFileName;
        let length = filename
            .iter()
            .position(|unit| *unit == 0)
            .ok_or("Attachment name is too long.")?;
        let name = String::from_utf16(&filename[..length]).map_err(err)?;
        safe_name(&name, index)?;
        let expected = (descriptor.dwFlags & FD_FILESIZE.0 as u32 != 0)
            .then_some(((descriptor.nFileSizeHigh as u64) << 32) | descriptor.nFileSizeLow as u64);
        if expected.is_some_and(|size| size > MAX_FILE as u64) {
            return Err("An attachment exceeds 16 MiB.".into());
        }
        if expected == Some(0) {
            files.push((name, Vec::new()));
            continue;
        }
        let mut request = format("FileContents");
        request.lindex = index as i32;
        request.tymed = (TYMED_HGLOBAL.0 | TYMED_ISTREAM.0) as u32;
        let medium = Medium(
            data.GetData(&request)
                .map_err(|_| "The source could not supply the attachment bytes.")?,
        );
        let content = if medium.0.tymed == TYMED_ISTREAM.0 as u32 {
            let stream = medium
                .0
                .u
                .pstm
                .as_ref()
                .ok_or("Missing attachment stream.")?;
            let mut result = Vec::new();
            let started = Instant::now();
            loop {
                if started.elapsed() > Duration::from_secs(15) {
                    return Err("Attachment read timed out.".into());
                }
                let mut buffer = [0u8; 65536];
                let mut read = 0;
                stream
                    .Read(
                        buffer.as_mut_ptr().cast(),
                        buffer.len() as u32,
                        Some(&mut read),
                    )
                    .ok()
                    .map_err(err)?;
                if read as usize > buffer.len() {
                    return Err("Invalid attachment stream size.".into());
                }
                if read == 0 {
                    break;
                }
                append(&mut result, &buffer[..read as usize])?;
            }
            if expected.is_some_and(|size| size != result.len() as u64) {
                return Err("The attachment stream was incomplete.".into());
            }
            result
        } else {
            let mut result = bytes(&medium, MAX_FILE + 65536)?;
            if let Some(size) = expected {
                if size > result.len() as u64 {
                    return Err("The attachment bytes were incomplete.".into());
                }
                result.truncate(size as usize);
            }
            result
        };
        total += content.len();
        if total > MAX_BATCH {
            return Err("Drop at most 32 MiB of attachments at once.".into());
        }
        files.push((name, content));
    }
    Ok(DropData::Files(files))
}

fn append(result: &mut Vec<u8>, bytes: &[u8]) -> Result<(), String> {
    if result.len().saturating_add(bytes.len()) > MAX_FILE {
        return Err("The imported file exceeds 16 MiB.".into());
    }
    result.extend_from_slice(bytes);
    Ok(())
}

fn safe_name(name: &str, index: usize) -> Result<String, String> {
    let name = name.trim_end_matches(['.', ' ']);
    if name.is_empty()
        || name.chars().count() > 180
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
    {
        return Err("The source supplied an invalid attachment filename.".into());
    }
    // Prefixes also make reserved DOS names and duplicate filenames harmless.
    Ok(format!("{:02}-{name}", index + 1))
}

pub fn materialize(data: DropData, root: &Path) -> Result<Vec<PathBuf>, String> {
    let files = match data {
        DropData::Paths(paths) => return Ok(paths),
        DropData::Files(files) => files,
        DropData::Download { url, name } => vec![fetch(url, name)?],
    };
    if files.is_empty() || files.len() > 20 {
        return Err("Drop 1–20 files at a time.".into());
    }
    let mut names = Vec::new();
    let mut total = 0usize;
    for (index, (name, bytes)) in files.iter().enumerate() {
        names.push(safe_name(name, index)?);
        if bytes.len() > MAX_FILE {
            return Err("An imported file exceeds 16 MiB.".into());
        }
        total = total.saturating_add(bytes.len());
    }
    if total > MAX_BATCH {
        return Err("Drop at most 32 MiB of files at once.".into());
    }
    std::fs::create_dir_all(root).map_err(err)?;
    if cache_size(root)?.saturating_add(total as u64) > MAX_CACHE {
        return Err(
            "Imported file storage is full. Open Imported files from the pet menu to manage it."
                .into(),
        );
    }
    let directory = root.join(new_id());
    std::fs::create_dir(&directory).map_err(err)?;
    let result = (|| {
        let mut paths = Vec::new();
        for ((_, bytes), name) in files.iter().zip(&names) {
            let path = directory.join(name);
            std::fs::write(&path, bytes).map_err(err)?;
            paths.push(path);
        }
        Ok(paths)
    })();
    if result.is_err() {
        for name in names {
            let _ = std::fs::remove_file(directory.join(name));
        }
        let _ = std::fs::remove_dir(directory);
    }
    result
}

fn ordinary(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0
}

fn cache_size(root: &Path) -> Result<u64, String> {
    if !ordinary(&std::fs::symlink_metadata(root).map_err(err)?) {
        return Err("The attachment cache must be a local directory.".into());
    }
    let mut size = 0u64;
    for (index, entry) in std::fs::read_dir(root).map_err(err)?.enumerate() {
        if index >= 256 {
            return Err("The attachment cache contains too many batches.".into());
        }
        let entry = entry.map_err(err)?;
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(err)?;
        if !ordinary(&metadata) || !metadata.is_dir() {
            continue;
        }
        for (count, file) in std::fs::read_dir(entry.path()).map_err(err)?.enumerate() {
            if count >= 20 {
                return Err("The attachment cache contains an unexpected directory.".into());
            }
            let file = file.map_err(err)?;
            let metadata = std::fs::symlink_metadata(file.path()).map_err(err)?;
            if !ordinary(&metadata) || !metadata.is_file() {
                return Err("The attachment cache contains an unexpected entry.".into());
            }
            size = size.saturating_add(metadata.len());
        }
    }
    Ok(size)
}

struct Internet(*mut std::ffi::c_void);
impl Drop for Internet {
    fn drop(&mut self) {
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}
impl Internet {
    fn new(handle: *mut std::ffi::c_void) -> Result<Self, String> {
        if handle.is_null() {
            Err("Could not start the attachment download.".into())
        } else {
            Ok(Self(handle))
        }
    }
}

unsafe fn header(request: &Internet, query: u32) -> String {
    let mut buffer = [0u16; 2048];
    let mut size = (buffer.len() * 2) as u32;
    if WinHttpQueryHeaders(
        request.0,
        query,
        None,
        Some(buffer.as_mut_ptr().cast()),
        &mut size,
        std::ptr::null_mut(),
    )
    .is_err()
    {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(size as usize / 2).min(buffer.len())])
        .trim_end_matches('\0')
        .to_owned()
}

fn fetch(mut url: tauri::Url, mut name: String) -> Result<(String, Vec<u8>), String> {
    unsafe {
        let access = if matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost")) {
            WINHTTP_ACCESS_TYPE_NO_PROXY
        } else {
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY
        };
        let session = Internet::new(WinHttpOpen(w!("Desktop-Buddy/0.4"), access, None, None, 0))?;
        WinHttpSetTimeouts(session.0, 5000, 5000, 5000, 5000)
            .map_err(|_| "Could not set download time limits.")?;
        let started = Instant::now();
        for redirect in 0..=3 {
            if started.elapsed() > Duration::from_secs(15) {
                return Err("Attachment download timed out.".into());
            }
            let host: Vec<_> = url
                .host_str()
                .ok_or("Missing download host.")?
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let connection = Internet::new(WinHttpConnect(
                session.0,
                PCWSTR(host.as_ptr()),
                url.port_or_known_default()
                    .ok_or("Missing download port.")?,
                0,
            ))?;
            let mut path = url.path().to_owned();
            if let Some(query) = url.query() {
                path.push('?');
                path.push_str(query);
            }
            let path: Vec<_> = path.encode_utf16().chain(Some(0)).collect();
            let flags = if url.scheme() == "https" {
                WINHTTP_FLAG_SECURE
            } else {
                WINHTTP_OPEN_REQUEST_FLAGS(0)
            };
            let request = Internet::new(WinHttpOpenRequest(
                connection.0,
                w!("GET"),
                PCWSTR(path.as_ptr()),
                None,
                None,
                std::ptr::null(),
                flags,
            ))?;
            WinHttpSetOption(
                Some(request.0),
                WINHTTP_OPTION_REDIRECT_POLICY,
                Some(&WINHTTP_OPTION_REDIRECT_POLICY_NEVER.to_ne_bytes()),
            )
            .map_err(err)?;
            WinHttpSetOption(
                Some(request.0),
                WINHTTP_OPTION_DISABLE_FEATURE,
                Some(&(WINHTTP_DISABLE_COOKIES | WINHTTP_DISABLE_AUTHENTICATION).to_ne_bytes()),
            )
            .map_err(err)?;
            WinHttpSendRequest(request.0, None, None, 0, 0, 0).map_err(|error| {
                format!(
                    "Attachment download failed ({error}). Use the source’s save/download action."
                )
            })?;
            WinHttpReceiveResponse(request.0, std::ptr::null_mut())
                .map_err(|_| "The source did not finish its download response.")?;
            let status = header(&request, WINHTTP_QUERY_STATUS_CODE)
                .parse::<u16>()
                .unwrap_or(0);
            if matches!(status, 301 | 302 | 303 | 307 | 308) && redirect < 3 {
                let next = url
                    .join(&header(&request, WINHTTP_QUERY_LOCATION))
                    .map_err(|_| "Invalid attachment redirect.")?;
                if !matches!(next.scheme(), "http" | "https")
                    || !next.username().is_empty()
                    || next.password().is_some()
                    || (url.scheme() == "https" && next.scheme() != "https")
                {
                    return Err("The attachment redirected to an unsupported address.".into());
                }
                url = next;
                continue;
            }
            if status != 200 {
                return Err(format!("Attachment download returned HTTP {status}. Save the file in its source app and drop it again."));
            }
            if header(&request, WINHTTP_QUERY_CONTENT_LENGTH)
                .parse::<u64>()
                .ok()
                .is_some_and(|size| size > MAX_FILE as u64)
            {
                return Err("The attachment download exceeds 16 MiB.".into());
            }
            let mime = header(&request, WINHTTP_QUERY_CONTENT_TYPE);
            let html_file = Path::new(&name)
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("html") || ext.eq_ignore_ascii_case("htm")
                });
            if mime.to_ascii_lowercase().starts_with("text/html") && !html_file {
                return Err("The source returned a web page instead of the attachment. Use its save/download action.".into());
            }
            if Path::new(&name).extension().is_none() {
                name.push_str(&format!(".{}", extension(&mime)));
            }
            let mut result = Vec::new();
            loop {
                if started.elapsed() > Duration::from_secs(15) {
                    return Err("Attachment download timed out.".into());
                }
                let mut buffer = [0u8; 65536];
                let mut read = 0;
                WinHttpReadData(
                    request.0,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                )
                .map_err(|_| "Attachment download was interrupted.")?;
                if read == 0 {
                    break;
                }
                append(&mut result, &buffer[..read as usize])?;
            }
            return Ok((name, result));
        }
        Err("The attachment redirected too many times.".into())
    }
}

#[cfg(test)]
#[path = "drop_data_tests.rs"]
mod tests;

use super::*;
use windows::Win32::Foundation::{DV_E_FORMATETC, E_FAIL, E_NOTIMPL, S_OK};

#[windows::core::implement(IDataObject)]
struct Fixture {
    values: std::cell::RefCell<Vec<(FORMATETC, Vec<u8>, bool)>>,
}
impl IDataObject_Impl for Fixture_Impl {
    fn GetData(&self, request: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        unsafe {
            let request = &*request;
            let values = self.values.borrow();
            let (_, value, stream) = values
                .iter()
                .find(|(format, _, stream)| {
                    format.cfFormat == request.cfFormat
                        && format.lindex == request.lindex
                        && request.tymed
                            & (if *stream {
                                TYMED_ISTREAM.0
                            } else {
                                TYMED_HGLOBAL.0
                            }) as u32
                            != 0
                })
                .ok_or_else(|| windows::core::Error::from_hresult(DV_E_FORMATETC))?;
            let u = if *stream {
                STGMEDIUM_0 {
                    pstm: std::mem::ManuallyDrop::new(Some(
                        SHCreateMemStream(Some(value))
                            .ok_or_else(|| windows::core::Error::from_hresult(E_FAIL))?,
                    )),
                }
            } else {
                let global = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, value.len().max(1))?;
                let pointer = GlobalLock(global);
                assert!(!pointer.is_null());
                std::ptr::copy_nonoverlapping(value.as_ptr(), pointer.cast::<u8>(), value.len());
                let _ = GlobalUnlock(global);
                STGMEDIUM_0 { hGlobal: global }
            };
            Ok(STGMEDIUM {
                tymed: (if *stream {
                    TYMED_ISTREAM.0
                } else {
                    TYMED_HGLOBAL.0
                }) as u32,
                u,
                ..Default::default()
            })
        }
    }
    fn QueryGetData(&self, _: *const FORMATETC) -> windows::core::HRESULT {
        // Some real sources advertise formats that they later cannot render.
        S_OK
    }
    fn SetData(
        &self,
        format: *const FORMATETC,
        medium: *const STGMEDIUM,
        release: windows::core::BOOL,
    ) -> windows::core::Result<()> {
        unsafe {
            let format = (*format).clone();
            let medium_ref = &*medium;
            let is_stream = medium_ref.tymed == TYMED_ISTREAM.0 as u32;
            let mut content = Vec::new();
            if is_stream {
                let stream = medium_ref.u.pstm.as_ref().unwrap();
                loop {
                    let mut buffer = [0u8; 128];
                    let mut read = 0;
                    stream
                        .Read(
                            buffer.as_mut_ptr().cast(),
                            buffer.len() as u32,
                            Some(&mut read),
                        )
                        .ok()?;
                    if read == 0 {
                        break;
                    }
                    content.extend_from_slice(&buffer[..read as usize]);
                }
            } else {
                let global = medium_ref.u.hGlobal;
                let length = GlobalSize(global);
                let pointer = GlobalLock(global);
                assert!(!pointer.is_null());
                content.extend_from_slice(std::slice::from_raw_parts(pointer.cast::<u8>(), length));
                let _ = GlobalUnlock(global);
            }
            self.values.borrow_mut().push((format, content, is_stream));
            if release.as_bool() {
                ReleaseStgMedium(&mut std::ptr::read(medium));
            }
            Ok(())
        }
    }
    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn GetCanonicalFormatEtc(
        &self,
        _: *const FORMATETC,
        _: *mut FORMATETC,
    ) -> windows::core::HRESULT {
        E_NOTIMPL
    }
    fn EnumFormatEtc(&self, _: u32) -> windows::core::Result<IEnumFORMATETC> {
        unsafe {
            SHCreateStdEnumFmtEtc(
                &self
                    .values
                    .borrow()
                    .iter()
                    .map(|(format, _, _)| format.clone())
                    .collect::<Vec<_>>(),
            )
        }
    }
    fn DAdvise(
        &self,
        _: *const FORMATETC,
        _: u32,
        _: windows::core::Ref<IAdviseSink>,
    ) -> windows::core::Result<u32> {
        Err(E_NOTIMPL.into())
    }
    fn DUnadvise(&self, _: u32) -> windows::core::Result<()> {
        Err(E_NOTIMPL.into())
    }
    fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> {
        Err(E_NOTIMPL.into())
    }
}

struct Ole;
impl Ole {
    unsafe fn new() -> Self {
        OleInitialize(None).unwrap();
        Self
    }
}
impl Drop for Ole {
    fn drop(&mut self) {
        unsafe {
            OleUninitialize();
        }
    }
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("buddy-import-test-{}", new_id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

unsafe fn data_object() -> IDataObject {
    Fixture {
        values: std::cell::RefCell::new(Vec::new()),
    }
    .into()
}

unsafe fn set_bytes(data: &IDataObject, request: FORMATETC, value: &[u8]) {
    let global = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, value.len().max(1)).unwrap();
    let pointer = GlobalLock(global);
    assert!(!pointer.is_null());
    std::ptr::copy_nonoverlapping(value.as_ptr(), pointer.cast::<u8>(), value.len());
    let _ = GlobalUnlock(global);
    let medium = Medium(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: global },
        ..Default::default()
    });
    data.SetData(&request, &medium.0, true).unwrap();
    std::mem::forget(medium);
}

fn pickle(values: &[(&str, String)]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend((values.len() as u32).to_le_bytes());
    for (name, value) in values {
        for text in [*name, value.as_str()] {
            let units: Vec<_> = text.encode_utf16().collect();
            payload.extend((units.len() as u32).to_le_bytes());
            for unit in units {
                payload.extend(unit.to_le_bytes());
            }
            while payload.len() % 4 != 0 {
                payload.push(0);
            }
        }
    }
    let mut result = (payload.len() as u32).to_le_bytes().to_vec();
    result.extend(payload);
    result
}

unsafe fn descriptors(data: &IDataObject, entries: &[(&str, u64)]) {
    let mut raw = (entries.len() as u32).to_le_bytes().to_vec();
    raw.resize(
        4 + entries.len() * std::mem::size_of::<FILEDESCRIPTORW>(),
        0,
    );
    for (index, (name, size)) in entries.iter().enumerate() {
        let mut descriptor = FILEDESCRIPTORW::default();
        descriptor.dwFlags = FD_FILESIZE.0 as u32;
        descriptor.nFileSizeLow = *size as u32;
        descriptor.nFileSizeHigh = (*size >> 32) as u32;
        let mut filename = [0u16; 260];
        for (slot, unit) in filename.iter_mut().zip(name.encode_utf16()) {
            *slot = unit;
        }
        descriptor.cFileName = filename;
        std::ptr::write_unaligned(
            raw.as_mut_ptr()
                .add(4 + index * std::mem::size_of::<FILEDESCRIPTORW>())
                .cast::<FILEDESCRIPTORW>(),
            descriptor,
        );
    }
    set_bytes(data, format("FileGroupDescriptorW"), &raw);
}

#[test]
fn chromium_editor_and_local_webview_paths_are_decoded_without_touching_sources() {
    unsafe {
        let _ole = Ole::new();
        let root = Directory::new();
        let source = root.0.join("猫 # 100%.txt");
        std::fs::write(&source, b"original").unwrap();
        let uri = tauri::Url::from_file_path(&source).unwrap().to_string();
        let data = data_object();
        let custom = pickle(&[("ResourceURLs", serde_json::to_string(&vec![uri]).unwrap())]);
        set_bytes(&data, format(CUSTOM), &custom);
        assert!(supported(&data));
        match read(&data).unwrap() {
            DropData::Paths(paths) => assert_eq!(paths, vec![source.clone()]),
            _ => panic!("Expected local editor paths"),
        }
        let code_files = serde_json::to_string(&vec![source.display().to_string()]).unwrap();
        assert!(matches!(
            parse_text("CodeFiles", &code_files).unwrap(),
            DropData::Paths(_)
        ));
        let path = source.to_string_lossy().replace('\\', "/");
        let editors = serde_json::json!([{"resource":{"scheme":"file","path":format!("/{path}")}}]);
        match parse_text("CodeEditors", &editors.to_string()).unwrap() {
            DropData::Paths(paths) => assert_eq!(paths, vec![source.clone()]),
            _ => panic!("Expected saved editor path"),
        }
        let file_uri = tauri::Url::from_file_path(&source).unwrap();
        let webview = format!(
            "https://file+.vscode-resource.vscode-cdn.net{}",
            file_uri.path()
        );
        assert_eq!(local_resource(&webview).unwrap(), source);
        assert_eq!(std::fs::read(&source).unwrap(), b"original");
    }
}

#[test]
fn virtual_files_read_indexed_global_and_stream_contents_and_keep_duplicate_names() {
    unsafe {
        let _ole = Ole::new();
        let root = Directory::new();
        let data = data_object();
        descriptors(&data, &[("menu.pdf", 5), ("menu.pdf", 6), ("empty.txt", 0)]);
        let mut request = format("FileContents");
        request.lindex = 0;
        set_bytes(&data, request.clone(), b"first");
        request.lindex = 1;
        request.tymed = TYMED_ISTREAM.0 as u32;
        let medium = Medium(STGMEDIUM {
            tymed: TYMED_ISTREAM.0 as u32,
            u: STGMEDIUM_0 {
                pstm: std::mem::ManuallyDrop::new(SHCreateMemStream(Some(b"second"))),
            },
            ..Default::default()
        });
        data.SetData(&request, &medium.0, true).unwrap();
        std::mem::forget(medium);
        let files = materialize(read(&data).unwrap(), &root.0).unwrap();
        assert_eq!(files.len(), 3);
        assert_ne!(files[0], files[1]);
        assert_eq!(std::fs::read(&files[0]).unwrap(), b"first");
        assert_eq!(std::fs::read(&files[1]).unwrap(), b"second");
        assert!(std::fs::read(&files[2]).unwrap().is_empty());
        assert_eq!(super::super::pet::validated(files.clone()).unwrap(), files);
    }
}

#[test]
fn invalid_or_missing_virtual_content_never_creates_a_partial_batch() {
    unsafe {
        let _ole = Ole::new();
        let root = Directory::new();
        let data = data_object();
        descriptors(&data, &[("valid.txt", 5), ("second.txt", 6)]);
        let mut request = format("FileContents");
        request.lindex = 0;
        set_bytes(&data, request, b"first");
        assert!(read(&data).is_err());
        assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 0);
        let malformed = data_object();
        set_bytes(
            &malformed,
            format("FileGroupDescriptorW"),
            &20u32.to_le_bytes(),
        );
        assert!(read(&malformed).is_err());
        assert!(materialize(
            DropData::Files(vec![
                ("ok.txt".into(), b"keep".to_vec()),
                ("../escape".into(), vec![])
            ]),
            &root.0
        )
        .is_err());
        assert_eq!(std::fs::read_dir(&root.0).unwrap().count(), 0);
        for name in ["", ".", "..", "C:\\bad", "bad/file", "bad\nfile"] {
            assert!(safe_name(name, 0).is_err());
        }
        assert_eq!(safe_name("CON", 0).unwrap(), "01-CON");
    }
}

#[test]
fn browser_png_data_urls_and_bitmaps_have_real_local_file_contents() {
    unsafe {
        let _ole = Ole::new();
        let root = Directory::new();
        let png = b"\x89PNG\r\n\x1a\nfixture";
        let data = data_object();
        set_bytes(&data, format("PNG"), png);
        let files = materialize(read(&data).unwrap(), &root.0).unwrap();
        assert!(std::fs::read(&files[0]).unwrap().starts_with(png));
        let embedded = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        );
        let files = materialize(parse_text("text/uri-list", &embedded).unwrap(), &root.0).unwrap();
        assert_eq!(std::fs::read(&files[0]).unwrap(), png);
        let mut dib = vec![0u8; 44];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&1u32.to_le_bytes());
        dib[8..12].copy_from_slice(&1u32.to_le_bytes());
        dib[12..14].copy_from_slice(&1u16.to_le_bytes());
        dib[14..16].copy_from_slice(&24u16.to_le_bytes());
        let bmp = bitmap_file(&dib).unwrap();
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(u32::from_le_bytes(bmp[10..14].try_into().unwrap()), 54);
        assert!(bitmap_file(&dib[..43]).is_err());
        assert!(custom_payloads(&[255, 255, 255, 255]).is_err());
        let mut over_limit = vec![0u8; MAX_FILE];
        assert!(append(&mut over_limit, &[0]).is_err());
        assert_eq!(over_limit.len(), MAX_FILE);
    }
}

#[test]
fn imported_copies_remain_available_after_the_hold_and_storage_is_accounted_for() {
    let root = Directory::new();
    let first = materialize(
        DropData::Files(vec![("old.txt".into(), b"old".to_vec())]),
        &root.0,
    )
    .unwrap();
    let second = materialize(
        DropData::Files(vec![("new.txt".into(), b"new".to_vec())]),
        &root.0,
    )
    .unwrap();
    assert_eq!(cache_size(&root.0).unwrap(), 6);
    assert_eq!(std::fs::read(&first[0]).unwrap(), b"old");
    assert_eq!(std::fs::read(&second[0]).unwrap(), b"new");
}

#[test]
fn download_url_uses_a_bounded_local_http_fixture_and_follows_relative_redirects() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let finished = stop.clone();
    let server = std::thread::spawn(move || {
        let started = Instant::now();
        let mut requests = Vec::new();
        while !finished.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(20) {
            match listener.accept() {
                Ok((mut socket, _)) => {
                    // Accepted sockets inherit nonblocking mode on Windows.
                    socket.set_nonblocking(false).unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    // TCP can split the request headers across packets. Closing with
                    // unread bytes can reset the socket before WinHTTP sees the reply.
                    let mut buffer = Vec::new();
                    while buffer.len() < 8192 && !buffer.windows(4).any(|part| part == b"\r\n\r\n") {
                        let mut chunk = [0u8; 1024];
                        match socket.read(&mut chunk) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => buffer.extend_from_slice(&chunk[..count]),
                        }
                    }
                    let request = String::from_utf8_lossy(&buffer).to_string();
                    let response = if request.starts_with("GET /start ") {
                        "HTTP/1.1 302 Found\r\nLocation: /file\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: 7\r\nConnection: close\r\n\r\npdfdata"
                    };
                    socket.write_all(response.as_bytes()).unwrap();
                    let _ = socket.shutdown(std::net::Shutdown::Write);
                    requests.push(request);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
        requests
    });
    let directory = Directory::new();
    let payload = parse_text(
        "DownloadURL",
        &format!("application/pdf:report.pdf:http://{address}/start"),
    )
    .unwrap();
    let result = materialize(payload, &directory.0);
    stop.store(true, Ordering::Relaxed);
    let requests = server.join().unwrap();
    let files = result.unwrap();
    assert_eq!(files[0].file_name().unwrap(), "01-report.pdf");
    assert_eq!(std::fs::read(&files[0]).unwrap(), b"pdfdata");
    assert_eq!(requests.len(), 2, "request lines: {:?}", requests.iter().map(|request| request.lines().next().unwrap_or("")).collect::<Vec<_>>());
    assert!(requests.iter().all(|request| !request
        .to_ascii_lowercase()
        .contains("authorization:")
        && !request.to_ascii_lowercase().contains("cookie:")));
}

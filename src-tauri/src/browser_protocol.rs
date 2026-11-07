use serde_json::Value;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
pub const LIMIT: u64 = 512 * 1024;
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}
pub fn root() -> Result<PathBuf, String> {
    std::env::var_os("LOCALAPPDATA")
        .map(|path| PathBuf::from(path).join("local.dharu.desktopbuddy/browser-fill"))
        .ok_or("Local application data is unavailable.".into())
}
pub fn read(path: &Path) -> Result<Value, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Browser message exceeded its size limit.".into());
    }
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
pub fn atomic(path: &Path, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Browser message exceeded its size limit.".into());
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = std::fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| error.to_string())?;
    std::fs::rename(&temporary, path).map_err(|error| error.to_string())
}
pub fn read_frame(input: &mut impl Read) -> Result<Value, String> {
    let mut length = [0u8; 4];
    input
        .read_exact(&mut length)
        .map_err(|error| error.to_string())?;
    let length = u32::from_le_bytes(length) as u64;
    if length == 0 || length > LIMIT {
        return Err("Invalid native message length.".into());
    }
    let mut bytes = vec![0; length as usize];
    input
        .read_exact(&mut bytes)
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
pub fn write_frame(output: &mut impl Write, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Native response too large.".into());
    }
    output
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| output.write_all(&bytes))
        .and_then(|_| output.flush())
        .map_err(|error| error.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_frames_are_bounded_and_never_accept_paths_as_ids() {
        let value = serde_json::json!({"question":"Café 猫"});
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &value).unwrap();
        assert_eq!(read_frame(&mut &bytes[..]).unwrap(), value);
        assert!(read_frame(&mut &(LIMIT as u32 + 1).to_le_bytes()[..]).is_err());
        assert!(read_frame(&mut &bytes[..6]).is_err());
        for value in ["../file", "", "C:\\test", "a/b"] {
            assert!(!identifier(value));
        }
    }
}

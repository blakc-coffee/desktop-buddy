//! Opt-in "Start with Windows" via the current user's Run key. No service, no admin rights.
//! The registry value itself is the source of truth, so the menu check always matches Windows.
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
            RRF_RT_REG_SZ,
        },
    },
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: &str = "DesktopBuddy";

/// The command Windows should run at sign-in: the quoted path of this executable.
fn command() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!("\"{}\"", exe.display()))
}

fn read(name: &str) -> Option<String> {
    let name = HSTRING::from(name);
    let mut buffer = [0u16; 1024];
    let mut bytes = std::mem::size_of_val(&buffer) as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            &name,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let chars = (bytes as usize / 2).saturating_sub(1).min(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..chars]))
}

fn write(name: &str, value: &str) -> Result<(), String> {
    let name = HSTRING::from(name);
    let data: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let status = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            &name,
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(format!("Windows refused the startup setting ({}).", status.0))
    }
}

fn delete(name: &str) -> Result<(), String> {
    let name = HSTRING::from(name);
    let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN_KEY, &name) };
    if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(format!("Windows refused to remove the startup setting ({}).", status.0))
    }
}

pub fn enabled() -> bool {
    read(VALUE).is_some()
}

pub fn set(on: bool) -> Result<(), String> {
    if on {
        write(VALUE, &command()?)
    } else {
        delete(VALUE)
    }
}

/// Buddy is portable: if autostart is on but points at an old location, point it here.
pub fn refresh() {
    if let (Some(current), Ok(expected)) = (read(VALUE), command()) {
        if !current.eq_ignore_ascii_case(&expected) {
            let _ = write(VALUE, &expected);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_value_round_trips_and_deletes_idempotently() {
        let name = format!("DesktopBuddyTest-{}", std::process::id());
        assert_eq!(read(&name), None);
        write(&name, "\"C:\\Buddy Test\\desktop-buddy.exe\"").unwrap();
        assert_eq!(read(&name).as_deref(), Some("\"C:\\Buddy Test\\desktop-buddy.exe\""));
        delete(&name).unwrap();
        assert_eq!(read(&name), None);
        delete(&name).unwrap();
    }

    #[test]
    fn command_quotes_the_current_executable() {
        let command = command().unwrap();
        assert!(command.starts_with('"') && command.ends_with('"'));
        assert!(command.to_ascii_lowercase().ends_with(".exe\""));
    }
}

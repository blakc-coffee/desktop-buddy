//! A small local error log and a "Copy diagnostics" report. Nothing is ever uploaded, and log
//! lines carry error messages only, never todo titles or file paths.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

/// The log rotates to buddy.log.1 once it passes 256 KB, so it never grows without bound.
const LIMIT: u64 = 262_144;

pub fn log(path: &Path, line: &str) {
    if fs::metadata(path).is_ok_and(|meta| meta.len() > LIMIT) {
        let _ = fs::rename(path, path.with_extension("log.1"));
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let line: String = line.chars().take(300).collect();
        let _ = writeln!(file, "{stamp} {line}");
    }
}

/// The last `lines` lines of the log, for the diagnostics report.
pub fn tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

pub fn report(version: &str, os: &str, ram_mb: u64, flags: &[(&str, bool)], tail: &str) -> String {
    let flags: Vec<String> = flags.iter().map(|(name, on)| format!("{name}={on}")).collect();
    format!(
        "Desktop Buddy {version}\nWindows {os}\nMemory {ram_mb} MB\nSettings {}\n--- last log lines ---\n{tail}",
        flags.join(" ")
    )
}

/// Records panics in the log before the default handler runs.
pub fn install_panic_hook(path: std::path::PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        log(&path, &format!("panic at {place}: {message}"));
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_rotates_at_256kb() {
        let dir = std::env::temp_dir().join(format!("buddy-diag-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("buddy.log");
        let line = "x".repeat(250);
        for _ in 0..1300 {
            log(&path, &line);
        }
        assert!(fs::metadata(&path).unwrap().len() <= LIMIT + 300);
        assert!(dir.join("buddy.log.1").exists());
        assert_eq!(tail(&path, 2).lines().count(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn report_has_only_what_it_was_given() {
        let text = report("0.4.0", "10.0.26200", 27, &[("quiet", false), ("agent", true)], "1 started");
        assert!(text.contains("Desktop Buddy 0.4.0"));
        assert!(text.contains("quiet=false agent=true"));
        assert!(text.ends_with("1 started"));
    }
}

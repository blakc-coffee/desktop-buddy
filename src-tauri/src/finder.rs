use std::{
    collections::{HashSet, VecDeque},
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use tauri::Manager;

const REPARSE_POINT: u32 = 0x400;
pub const MAX_RESULTS: usize = 8;

pub fn roots(app: &tauri::AppHandle, saved: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let paths = app.path();
    let defaults = [
        paths.home_dir().map(|home| home.join("anything")),
        paths.download_dir(),
        paths.document_dir(),
    ];
    let mut roots: Vec<_> = defaults.into_iter().flatten().chain(saved).collect();
    if let Ok(config) = paths.config_dir() {
        for product in ["Antigravity", "Antigravity IDE"] {
            let storage = config.join(product).join("User").join("workspaceStorage");
            if let Ok(entries) = fs::read_dir(storage) {
                roots.extend(entries.flatten().take(256).filter_map(|entry| {
                    let file = fs::File::open(entry.path().join("workspace.json")).ok()?;
                    use std::io::Read;
                    let value: serde_json::Value =
                        serde_json::from_reader(file.take(32_768)).ok()?;
                    let uri = value.get("folder")?.as_str()?;
                    local_folder_uri(uri)
                }));
            }
        }
    }
    let mut seen = HashSet::new();
    if let Ok(home) = paths.home_dir() {
        roots.push(home);
    }
    let drives = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
    for index in 0..26 {
        if drives & (1 << index) != 0 {
            roots.push(PathBuf::from(format!("{}:\\", char::from(b'A' + index))));
        }
    }
    roots.retain(|path| path.is_absolute() && seen.insert(path_key(path)));
    roots
}

fn local_folder_uri(uri: &str) -> Option<PathBuf> {
    uri.strip_prefix("file:///")?;
    tauri::Url::parse(uri).ok()?.to_file_path().ok()
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().to_lowercase()
}

pub fn match_rank(name: &str, query: &str) -> Option<u8> {
    let name = name.to_lowercase();
    if name == query {
        Some(0)
    } else if name.starts_with(query) {
        Some(1)
    } else if name.contains(query) {
        Some(2)
    } else if query.split_whitespace().all(|word| name.contains(word)) {
        Some(3)
    } else {
        None
    }
}

pub fn find(
    roots: Vec<PathBuf>,
    query: &str,
    current: impl Fn() -> bool,
) -> (Vec<super::SearchResult>, bool) {
    let deadline = Instant::now() + Duration::from_millis(450);
    let mut pending: VecDeque<_> = roots.into_iter().map(|path| (path, 0)).collect();
    let mut visited = HashSet::new();
    let mut matches = Vec::new();
    let mut entries = 0;
    let mut limited = false;
    let mut results_seen = HashSet::new();
    'search: while let Some((path, depth)) = pending.pop_front() {
        if !current() || Instant::now() >= deadline {
            limited = true;
            break;
        }
        if !visited.insert(path_key(&path)) {
            continue;
        }
        let Ok(children) = fs::read_dir(&path) else {
            continue;
        };
        let root = (depth == 0).then(|| (path.clone(), fs::symlink_metadata(&path)));
        let candidates = root.into_iter().chain(
            children
                .flatten()
                .map(|entry| (entry.path(), entry.metadata())),
        );
        for (candidate, metadata) in candidates {
            if !current() || Instant::now() >= deadline || entries >= 30_000 {
                limited = true;
                break 'search;
            }
            entries += 1;
            let Some(name) = candidate
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            let rank = match_rank(&name, query);
            let Ok(metadata) = metadata else { continue };
            let folder = metadata.is_dir();
            let linked = metadata.file_attributes() & REPARSE_POINT != 0;
            if folder
                && candidate != path
                && !linked
                && !matches!(name.as_str(), ".git" | "node_modules" | "target" | ".venv")
            {
                if depth < 12 {
                    pending.push_back((candidate.clone(), depth + 1));
                } else {
                    limited = true;
                }
            }
            if let Some(rank) = rank.filter(|_| results_seen.insert(path_key(&candidate))) {
                matches.push((rank, name, candidate, folder));
            }
        }
    }
    matches.sort_unstable();
    let rows = matches
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, name, path, folder)| super::SearchResult {
            name,
            path: path.to_string_lossy().into_owned(),
            folder,
        })
        .collect();
    (rows, limited)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antigravity_local_workspace_uris_decode_windows_paths() {
        assert_eq!(
            local_folder_uri("file:///c%3A/Users/Test/Project%20One"),
            Some(PathBuf::from(r"c:\Users\Test\Project One"))
        );
        assert_eq!(local_folder_uri("vscode-remote://ssh/work"), None);
    }

    #[test]
    fn direct_names_rank_before_partial_and_reordered_matches() {
        assert_eq!(match_rank("Project", "project"), Some(0));
        assert_eq!(match_rank("Project plan.txt", "project"), Some(1));
        assert_eq!(match_rank("My project plan.txt", "project"), Some(2));
        assert_eq!(match_rank("Project plan.txt", "plan project"), Some(3));
        assert_eq!(match_rank("Project plan.txt", "project report"), None);
    }

    #[test]
    fn finds_nested_files_and_folders_across_overlapping_roots() {
        let root = std::env::temp_dir().join(format!(
            "buddy-finder-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let nested = root.join("projects");
        let folder = nested.join("cosmo");
        let file = nested.join("cosmo notes.txt");
        fs::create_dir_all(&folder).unwrap();
        fs::write(&file, "").unwrap();
        let (rows, limited) = find(vec![root.clone(), nested], "cosmo", || true);
        fs::remove_file(file).unwrap();
        fs::remove_dir(folder).unwrap();
        fs::remove_dir(root.join("projects")).unwrap();
        fs::remove_dir(root).unwrap();
        assert!(!limited);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "cosmo");
        assert!(rows[0].folder);
        assert_eq!(rows[1].name, "cosmo notes.txt");
        assert!(!rows[1].folder);
    }
}

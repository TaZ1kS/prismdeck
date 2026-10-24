use std::fs;
use std::path::Path;

/// Read a UTF-8-ish text file with an upper size bound. Prism config files
/// are small; refusing giant files protects the UI from a bad symlink or a
/// corrupted file.
pub fn read_text_limited(path: &Path, max_bytes: u64) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > max_bytes {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Read a directory's entries sorted by file name, so scans are deterministic.
pub fn sorted_dir_entries(path: &Path) -> Vec<fs::DirEntry> {
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = read_dir.flatten().collect();
    entries.sort_by_key(|entry| entry.file_name());
    entries
}

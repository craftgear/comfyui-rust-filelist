use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

fn is_allowed(entry: &DirEntry, excluded: &HashSet<String>) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    match entry.file_name().to_str() {
        Some(name) => !excluded.contains(name),
        None => true,
    }
}

fn mtime_seconds(path: &Path) -> Option<f64> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs_f64())
}

fn file_extension(path: &Path) -> String {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => format!(".{}", ext.to_lowercase()),
        None => String::new(),
    }
}

#[pyfunction]
fn scan_model_folders(
    folders: Vec<String>,
    extensions: Vec<String>,
    excluded_dir_names: Vec<String>,
) -> PyResult<(Vec<String>, HashMap<String, f64>)> {
    let mut files: HashSet<String> = HashSet::new();
    let mut dirs: HashMap<String, f64> = HashMap::new();

    let extension_set: HashSet<String> = extensions.into_iter().map(|ext| ext.to_lowercase()).collect();
    let excluded_set: HashSet<String> = excluded_dir_names.into_iter().collect();

    for folder in folders {
        let root = Path::new(&folder);
        if !root.is_dir() {
            continue;
        }

        if let Some(mtime) = mtime_seconds(root) {
            dirs.insert(folder.clone(), mtime);
        }

        let walker = WalkDir::new(root).follow_links(true).into_iter().filter_entry(|e| is_allowed(e, &excluded_set));
        for entry in walker {
            let entry = match entry {
                Ok(item) => item,
                Err(_) => continue,
            };

            if entry.depth() == 0 {
                continue;
            }

            let entry_path = entry.path();
            if entry.file_type().is_dir() {
                if let Some(mtime) = mtime_seconds(entry_path) {
                    dirs.insert(entry_path.to_string_lossy().to_string(), mtime);
                }
                continue;
            }

            if !extension_set.is_empty() {
                let ext = file_extension(entry_path);
                if !extension_set.contains(&ext) {
                    continue;
                }
            }

            let relative_path = entry_path.strip_prefix(root).unwrap_or(entry_path);
            let relative_str = relative_path.to_string_lossy().to_string();
            files.insert(relative_str);
        }
    }

    let mut files_list: Vec<String> = files.into_iter().collect();
    files_list.sort();

    Ok((files_list, dirs))
}

#[pymodule]
fn comfyui_rust_filelist(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_model_folders, m)?)?;
    Ok(())
}

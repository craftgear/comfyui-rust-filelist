use pyo3::prelude::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::IsTerminal;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};
use walkdir::{DirEntry, WalkDir};

const SPINNER_TICK_MILLIS: u64 = 80;
const SPINNER_TICK_STRINGS: [&str; 10] = [
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
];

fn start_spinner(message: &str) -> Option<indicatif::ProgressBar> {
    if !std::io::stdout().is_terminal() {
        // ログ汚染を避けるため、TTY のときだけ表示する
        return None;
    }

    let spinner = indicatif::ProgressBar::new_spinner();
    let style = indicatif::ProgressStyle::with_template("{spinner} {msg}").ok()?;
    spinner.set_style(style.tick_strings(&SPINNER_TICK_STRINGS));
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(SPINNER_TICK_MILLIS));
    Some(spinner)
}

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
    let spinner = start_spinner("Scanning model folders...");
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

        let walker = WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| is_allowed(e, &excluded_set));

        let (folder_files, folder_dirs) = walker
            .par_bridge()
            .fold(
                || (HashSet::new(), HashMap::new()),
                |mut acc, entry| {
                    let entry = match entry {
                        Ok(item) => item,
                        Err(_) => return acc,
                    };

                    if entry.depth() == 0 {
                        return acc;
                    }

                    let entry_path = entry.path();
                    if entry.file_type().is_dir() {
                        if let Some(mtime) = mtime_seconds(entry_path) {
                            acc.1
                                .insert(entry_path.to_string_lossy().to_string(), mtime);
                        }
                        return acc;
                    }

                    if !extension_set.is_empty() {
                        let ext = file_extension(entry_path);
                        if !extension_set.contains(&ext) {
                            return acc;
                        }
                    }

                    let relative_path = entry_path.strip_prefix(root).unwrap_or(entry_path);
                    let relative_str = relative_path.to_string_lossy().to_string();
                    acc.0.insert(relative_str);

                    acc
                },
            )
            .reduce(
                || (HashSet::new(), HashMap::new()),
                |mut left, right| {
                    left.0.extend(right.0);
                    left.1.extend(right.1);
                    left
                },
            );

        files.extend(folder_files);
        dirs.extend(folder_dirs);
    }

    let mut files_list: Vec<String> = files.into_iter().collect();
    files_list.sort();

    if let Some(spinner) = spinner {
        spinner.finish_and_clear();
    }

    Ok((files_list, dirs))
}

#[pymodule]
fn comfyui_fast_filelist(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_model_folders, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::scan_model_folders;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir() -> PathBuf {
        let mut path = env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("comfyui_fast_filelist_test_{}", unique));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn spinner_tick_strings_match_expected() {
        assert_eq!(
            super::SPINNER_TICK_STRINGS,
            ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        );
    }

    #[test]
    fn scan_model_folders_filters_and_collects() {
        let root = create_temp_dir();
        let keep_dir = root.join("models");
        let skip_dir = root.join("skip");
        fs::create_dir_all(&keep_dir).unwrap();
        fs::create_dir_all(&skip_dir).unwrap();

        fs::write(root.join("a.safetensors"), "a").unwrap();
        fs::write(root.join("b.txt"), "b").unwrap();
        fs::write(keep_dir.join("c.SAFETENSORS"), "c").unwrap();
        fs::write(skip_dir.join("d.safetensors"), "d").unwrap();

        let result = scan_model_folders(
            vec![root.to_string_lossy().to_string()],
            vec![".safetensors".to_string()],
            vec!["skip".to_string()],
        )
        .unwrap();

        let files = result.0;
        let dirs = result.1;
        let expected_nested = format!("models{}c.safetensors", std::path::MAIN_SEPARATOR);

        assert_eq!(files, vec!["a.safetensors".to_string(), expected_nested]);
        assert!(dirs.contains_key(&root.to_string_lossy().to_string()));
        assert!(dirs.contains_key(&keep_dir.to_string_lossy().to_string()));
        assert!(!dirs.contains_key(&skip_dir.to_string_lossy().to_string()));

        fs::remove_dir_all(root).unwrap();
    }
}

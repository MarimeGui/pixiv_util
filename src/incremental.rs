use std::{fs::read_dir, path::Path};

use anyhow::Result;

// TODO: This is not very smart or efficient

pub fn list_all_files(path: &Path) -> Result<Vec<String>> {
    let mut paths = Vec::new();

    let iterator = read_dir(path)?;
    for e in iterator {
        let e = e?;
        if e.file_type()?.is_file() {
            paths.push(e.path().display().to_string());
        } else {
            paths.extend_from_slice(&list_all_files(&e.path())?)
        }
    }

    Ok(paths)
}

pub fn is_illust_in_files(id: &str, files: &[String]) -> bool {
    let mut found = false;

    for name in files {
        if name.contains(id) {
            found = true;
            break;
        }
    }

    found
}

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_WRITE: AtomicU64 = AtomicU64::new(0);

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let (temporary, mut file) = loop {
        let temporary = path.with_extension(format!(
            "{}-{}.tmp",
            std::process::id(),
            NEXT_WRITE.fetch_add(1, Ordering::Relaxed)
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    let result = result.and_then(|_| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_files_and_cleans_up_failed_replacements() {
        let root = std::env::temp_dir().join(format!("sml2_atomic_{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("save");
        atomic_write(&path, b"previous").unwrap();
        atomic_write(&path, b"next").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"next");
        let blocked = root.join("directory");
        fs::create_dir_all(&blocked).unwrap();
        fs::write(blocked.join("previous"), b"keep").unwrap();
        assert!(atomic_write(&blocked, b"replacement").is_err());
        assert_eq!(fs::read(blocked.join("previous")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}

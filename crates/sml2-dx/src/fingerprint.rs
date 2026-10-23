//! Content identity of the extracted base assets a DX installation is built for.

use crate::ImportError;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

/// Hash the actual base bundle, including relative paths and file lengths.
/// Symlinks are rejected so cache identity cannot silently follow external data.
pub fn base_fingerprint(root: &Path) -> Result<String, ImportError> {
    fingerprint_with_limits(root, 32, 100_000, 512 * 1024 * 1024)
}

fn fingerprint_with_limits(
    root: &Path,
    max_depth: usize,
    max_files: usize,
    max_bytes: u64,
) -> Result<String, ImportError> {
    let bad = |s: &str| ImportError::InvalidBundle(s.into());
    fn collect(
        root: &Path,
        dir: &Path,
        files: &mut Vec<std::path::PathBuf>,
        depth: usize,
        max_depth: usize,
        max_files: usize,
    ) -> Result<(), ImportError> {
        if depth > max_depth {
            return Err(ImportError::InvalidBundle(
                "base asset directories exceed depth limit".into(),
            ));
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(ImportError::InvalidBundle(
                    "base assets contain a symlink".into(),
                ));
            }
            if kind.is_dir() {
                collect(root, &entry.path(), files, depth + 1, max_depth, max_files)?;
            } else if kind.is_file() {
                if files.len() >= max_files {
                    return Err(ImportError::InvalidBundle(
                        "base assets exceed file count limit".into(),
                    ));
                }
                files.push(entry.path().strip_prefix(root).unwrap().to_owned());
            } else {
                return Err(ImportError::InvalidBundle(
                    "base assets contain a nonregular entry".into(),
                ));
            }
        }
        Ok(())
    }
    if !fs::symlink_metadata(root)?.is_dir() {
        return Err(bad("base asset root must be a directory"));
    }
    if !root.join("levels/level01.manifest.json").is_file() {
        return Err(ImportError::InvalidBundle("existing extracted base assets are required; choose the extracted directory with --assets".into()));
    }
    let mut files = Vec::new();
    collect(root, root, &mut files, 0, max_depth, max_files)?;
    files.sort();
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0; 64 * 1024];
    for path in files {
        // Join components so literal backslashes in Unix filenames retain their
        // identity rather than colliding with a directory separator.
        let name = path
            .components()
            .map(|c| {
                c.as_os_str()
                    .to_str()
                    .ok_or_else(|| bad("non-UTF8 asset path"))
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        let absolute = root.join(&path);
        let metadata = fs::symlink_metadata(&absolute)?;
        if !metadata.is_file() {
            return Err(bad("base asset file changed type"));
        }
        let length = metadata.len();
        total = total
            .checked_add(length)
            .filter(|&n| n <= max_bytes)
            .ok_or_else(|| bad("base assets exceed byte limit"))?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update(length.to_le_bytes());
        let mut file = fs::File::open(&absolute)?;
        let mut read = 0u64;
        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            read += count as u64;
            if read > length {
                return Err(bad("base asset changed during fingerprinting"));
            }
            hash.update(&buffer[..count]);
        }
        if read != length {
            return Err(bad("base asset changed during fingerprinting"));
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn fingerprint_rejects_nested_and_required_file_symlinks() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("levels")).unwrap();
        fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
        std::os::unix::fs::symlink("levels/level01.manifest.json", root.path().join("alias"))
            .unwrap();
        assert!(base_fingerprint(root.path()).is_err());
        fs::remove_file(root.path().join("alias")).unwrap();
        fs::remove_file(root.path().join("levels/level01.manifest.json")).unwrap();
        fs::write(root.path().join("manifest"), b"{}").unwrap();
        std::os::unix::fs::symlink(
            "../manifest",
            root.path().join("levels/level01.manifest.json"),
        )
        .unwrap();
        assert!(base_fingerprint(root.path()).is_err());
    }

    #[test]
    fn fingerprint_streaming_and_resource_limits() {
        let root = tempfile::tempdir().unwrap();
        assert!(base_fingerprint(root.path()).is_err());
        fs::create_dir(root.path().join("levels")).unwrap();
        fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
        assert!(fingerprint_with_limits(root.path(), 0, 100, 1024).is_err());
        assert!(fingerprint_with_limits(root.path(), 32, 0, 1024).is_err());
        assert!(fingerprint_with_limits(root.path(), 32, 100, 1).is_err());
        assert!(fingerprint_with_limits(root.path(), 1, 1, 2).is_ok());
        let large = root.path().join("large.bin");
        let bytes = vec![0x5au8; 64 * 1024 + 17];
        fs::write(&large, &bytes).unwrap();
        let original = base_fingerprint(root.path()).unwrap();
        let mut bytes = bytes;
        bytes[64 * 1024 + 16] ^= 1;
        fs::write(&large, &bytes).unwrap();
        assert_ne!(base_fingerprint(root.path()).unwrap(), original);
        fs::write(&large, vec![0x5au8; 64 * 1024 + 17]).unwrap();
        fs::rename(&large, root.path().join("renamed.bin")).unwrap();
        assert_ne!(base_fingerprint(root.path()).unwrap(), original);
    }

    #[cfg(unix)]
    #[test]
    fn fingerprint_distinguishes_literal_backslashes_from_path_separators() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("levels")).unwrap();
        fs::write(root.path().join("levels/level01.manifest.json"), b"{}").unwrap();
        fs::write(root.path().join("levels\\extra"), b"a").unwrap();
        let literal = base_fingerprint(root.path()).unwrap();
        fs::rename(
            root.path().join("levels\\extra"),
            root.path().join("levels/extra"),
        )
        .unwrap();
        assert_ne!(base_fingerprint(root.path()).unwrap(), literal);
    }

    #[test]
    fn base_identity_includes_names_and_contents() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("levels")).unwrap();
        let file = dir.path().join("levels/level01.manifest.json");
        fs::write(&file, b"{}").unwrap();
        let original = base_fingerprint(dir.path()).unwrap();
        fs::write(&file, b"[]").unwrap();
        assert_ne!(base_fingerprint(dir.path()).unwrap(), original);
        fs::write(&file, b"{}").unwrap();
        fs::write(dir.path().join("extra"), b"").unwrap();
        assert_ne!(base_fingerprint(dir.path()).unwrap(), original);
    }
}

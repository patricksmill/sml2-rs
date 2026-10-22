//! Transactional, local-only presentation bundles. Cache names are derived from
//! content identity; neither ROM metadata nor manifests supply filesystem paths.

use crate::{sha256, ImportError, BUNDLE_SCHEMA, IMPORTER_VERSION, ROM_SHA256};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

pub const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const MAX_MANIFEST_BYTES: usize = 4096;
static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CacheKey {
    pub source_sha256: String,
    pub base_sha256: String,
    pub importer_version: u32,
    pub schema_version: u32,
}

impl CacheKey {
    pub fn new(base_sha256: String) -> Self {
        Self {
            source_sha256: ROM_SHA256.into(),
            base_sha256,
            importer_version: IMPORTER_VERSION,
            schema_version: BUNDLE_SCHEMA,
        }
    }

    pub fn validate(&self) -> Result<(), ImportError> {
        if !valid_digest(&self.source_sha256) || !valid_digest(&self.base_sha256) {
            return Err(invalid("cache identity requires lowercase SHA-256 digests"));
        }
        Ok(())
    }

    pub fn directory_name(&self) -> String {
        // Serializing this fixed struct of strings and integers cannot fail.
        sha256(&serde_json::to_vec(self).expect("serializable cache identity"))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    key: CacheKey,
    payload_sha256: String,
}

fn invalid(message: &str) -> ImportError {
    ImportError::InvalidBundle(message.into())
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn check_cancel(cancelled: &AtomicBool) -> Result<(), ImportError> {
    if cancelled.load(Ordering::Acquire) {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}

fn read_bounded(path: &Path, cap: usize) -> Result<Vec<u8>, ImportError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > cap as u64 {
        return Err(invalid("cache file is not regular or exceeds size limit"));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err(invalid("cache file exceeds size limit"));
    }
    Ok(bytes)
}

fn load_directory(
    directory: &Path,
    key: &CacheKey,
    validate: &impl Fn(&[u8]) -> Result<(), ImportError>,
) -> Result<Vec<u8>, ImportError> {
    if !fs::symlink_metadata(directory)?.is_dir() {
        return Err(invalid("cache entry is not a directory"));
    }
    let manifest: Manifest = serde_json::from_slice(&read_bounded(
        &directory.join("manifest.json"),
        MAX_MANIFEST_BYTES,
    )?)
    .map_err(|_| invalid("malformed cache manifest"))?;
    manifest.key.validate()?;
    if manifest.key != *key || !valid_digest(&manifest.payload_sha256) {
        return Err(invalid("cache manifest identity mismatch"));
    }
    let payload = read_bounded(&directory.join("bundle.json"), MAX_PAYLOAD_BYTES)?;
    if sha256(&payload) != manifest.payload_sha256 {
        return Err(invalid("cache payload digest mismatch"));
    }
    validate(&payload)?;
    Ok(payload)
}

/// Load exactly the caller's identity. Callers enforce supported importer/source
/// versions and validate the payload schema; stale identities never fall back.
pub fn load(
    root: &Path,
    key: &CacheKey,
    validate: impl Fn(&[u8]) -> Result<(), ImportError>,
) -> Result<Vec<u8>, ImportError> {
    key.validate()?;
    load_directory(&root.join(key.directory_name()), key, &validate)
}

struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_stage(root: &Path) -> Result<Stage, ImportError> {
    loop {
        let path = root.join(format!(
            ".dx-import-{}-{}",
            std::process::id(),
            NEXT_STAGE.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(Stage(path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

fn write_synced(path: &Path, bytes: &[u8], cancelled: &AtomicBool) -> Result<(), ImportError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    for chunk in bytes.chunks(64 * 1024) {
        check_cancel(cancelled)?;
        file.write_all(chunk)?;
    }
    file.sync_all()?;
    check_cancel(cancelled)
}

fn reuse(
    directory: &Path,
    key: &CacheKey,
    payload: &[u8],
    validate: &impl Fn(&[u8]) -> Result<(), ImportError>,
    cancelled: &AtomicBool,
) -> Result<PathBuf, ImportError> {
    let installed = load_directory(directory, key, validate)?;
    check_cancel(cancelled)?;
    if installed != payload {
        return Err(invalid(
            "different bundle already exists for this cache identity",
        ));
    }
    Ok(directory.to_owned())
}

/// Validate and atomically publish an immutable cache entry. Cancellation before
/// the atomic rename leaves published entries untouched; rename is the commit point.
pub fn publish(
    root: &Path,
    key: &CacheKey,
    payload: &[u8],
    validate: impl Fn(&[u8]) -> Result<(), ImportError>,
    cancelled: &AtomicBool,
) -> Result<PathBuf, ImportError> {
    check_cancel(cancelled)?;
    key.validate()?;
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(invalid("bundle exceeds cache size limit"));
    }
    validate(payload)?;
    check_cancel(cancelled)?;
    fs::create_dir_all(root)?;
    let destination = root.join(key.directory_name());
    // symlink_metadata detects dangling symlinks as existing invalid entries.
    match fs::symlink_metadata(&destination) {
        Ok(_) => return reuse(&destination, key, payload, &validate, cancelled),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let stage = create_stage(root)?;
    check_cancel(cancelled)?;
    let manifest = Manifest {
        key: key.clone(),
        payload_sha256: sha256(payload),
    };
    let manifest = serde_json::to_vec(&manifest).map_err(|e| invalid(&e.to_string()))?;
    write_synced(&stage.0.join("bundle.json"), payload, cancelled)?;
    write_synced(&stage.0.join("manifest.json"), &manifest, cancelled)?;
    let staged = load_directory(&stage.0, key, &validate)?;
    if staged != payload {
        return Err(invalid("staged bundle changed during publication"));
    }
    check_cancel(cancelled)?;
    match fs::rename(&stage.0, &destination) {
        Ok(()) => Ok(destination),
        Err(error) => {
            // Another publisher may have won the same immutable key. Its fully
            // validated payload must agree before treating the race as success.
            if fs::symlink_metadata(&destination).is_ok() {
                reuse(&destination, key, payload, &validate, cancelled)
            } else {
                Err(error.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key() -> CacheKey {
        CacheKey::new(sha256(b"base assets"))
    }
    fn accept(_: &[u8]) -> Result<(), ImportError> {
        Ok(())
    }
    fn running() -> AtomicBool {
        AtomicBool::new(false)
    }
    fn entries(root: &Path) -> usize {
        fs::read_dir(root).unwrap().count()
    }

    #[test]
    fn reuse_requires_exact_identity_and_payload() {
        let root = tempfile::tempdir().unwrap();
        let key = key();
        let path = publish(root.path(), &key, b"{}", accept, &running()).unwrap();
        assert_eq!(
            publish(root.path(), &key, b"{}", accept, &running()).unwrap(),
            path
        );
        assert!(publish(root.path(), &key, b"[]", accept, &running()).is_err());
        assert_eq!(load(root.path(), &key, accept).unwrap(), b"{}");
        assert_eq!(entries(root.path()), 1);
        let mut changed = key.clone();
        changed.importer_version += 1;
        assert_ne!(changed.directory_name(), key.directory_name());
        assert!(load(root.path(), &changed, accept).is_err());
        changed = key.clone();
        changed.schema_version += 1;
        assert_ne!(changed.directory_name(), key.directory_name());
        changed = key.clone();
        changed.base_sha256 = sha256(b"other base");
        assert_ne!(changed.directory_name(), key.directory_name());
        let mut malformed = key;
        malformed.base_sha256 = "../bad".into();
        assert!(publish(root.path(), &malformed, b"{}", accept, &running()).is_err());
    }

    #[test]
    fn corruption_is_preserved_and_never_reused() {
        for corrupt_manifest in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = publish(root.path(), &key(), b"{}", accept, &running()).unwrap();
            let file = path.join(if corrupt_manifest {
                "manifest.json"
            } else {
                "bundle.json"
            });
            fs::write(&file, b"corrupt").unwrap();
            assert!(load(root.path(), &key(), accept).is_err());
            assert!(publish(root.path(), &key(), b"{}", accept, &running()).is_err());
            assert_eq!(fs::read(file).unwrap(), b"corrupt");
            assert_eq!(entries(root.path()), 1);
        }
    }

    #[test]
    fn cancellation_and_validation_failures_cleanup_staging() {
        let root = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(true);
        assert_eq!(
            publish(root.path(), &key(), b"{}", accept, &cancelled),
            Err(ImportError::Cancelled)
        );
        assert_eq!(entries(root.path()), 0);
        let counter = AtomicU64::new(0);
        let fail_readback = |_: &[u8]| {
            if counter.fetch_add(1, Ordering::Relaxed) == 0 {
                Ok(())
            } else {
                Err(invalid("invalid staged bundle"))
            }
        };
        assert!(publish(root.path(), &key(), b"{}", fail_readback, &running()).is_err());
        assert_eq!(entries(root.path()), 0);
        let cancelled = running();
        let validations = AtomicU64::new(0);
        let cancel_readback = |_: &[u8]| {
            if validations.fetch_add(1, Ordering::Relaxed) != 0 {
                cancelled.store(true, Ordering::Release);
            }
            Ok(())
        };
        assert_eq!(
            publish(root.path(), &key(), b"{}", cancel_readback, &cancelled),
            Err(ImportError::Cancelled)
        );
        assert_eq!(entries(root.path()), 0);
        publish(root.path(), &key(), b"{}", accept, &running()).unwrap();
        assert!(publish(
            root.path(),
            &key(),
            b"[]",
            |_| Err(invalid("reject")),
            &running()
        )
        .is_err());
        assert_eq!(
            publish(root.path(), &key(), b"{}", accept, &cancelled),
            Err(ImportError::Cancelled)
        );
        assert_eq!(load(root.path(), &key(), accept).unwrap(), b"{}");
    }

    #[test]
    fn concurrent_publishers_reuse_one_complete_entry() {
        let root = tempfile::tempdir().unwrap();
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        publish(root.path(), &key(), b"{}", accept, &running()).unwrap()
                    })
                })
                .collect();
            for handle in handles {
                assert_eq!(
                    handle.join().unwrap(),
                    root.path().join(key().directory_name())
                );
            }
        });
        assert_eq!(entries(root.path()), 1);
        assert_eq!(load(root.path(), &key(), accept).unwrap(), b"{}");
    }

    #[test]
    fn payload_and_manifest_limits_are_enforced() {
        let root = tempfile::tempdir().unwrap();
        assert!(publish(
            root.path(),
            &key(),
            &vec![0; MAX_PAYLOAD_BYTES + 1],
            accept,
            &running()
        )
        .is_err());
        assert_eq!(entries(root.path()), 0);
        let path = publish(root.path(), &key(), b"{}", accept, &running()).unwrap();
        fs::write(
            path.join("manifest.json"),
            vec![b' '; MAX_MANIFEST_BYTES + 1],
        )
        .unwrap();
        assert!(load(root.path(), &key(), accept).is_err());
    }

    #[test]
    fn manifest_identity_and_digest_fields_are_validated() {
        let root = tempfile::tempdir().unwrap();
        let path = publish(root.path(), &key(), b"{}", accept, &running()).unwrap();
        let manifest_path = path.join("manifest.json");
        let original = fs::read(&manifest_path).unwrap();
        for field in ["payload_sha256", "base_sha256"] {
            let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
            if field == "payload_sha256" {
                manifest[field] = "F".repeat(64).into();
            } else {
                manifest["key"][field] = sha256(b"other base").into();
            }
            fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
            assert!(load(root.path(), &key(), accept).is_err());
            assert!(publish(root.path(), &key(), b"{}", accept, &running()).is_err());
        }
    }
}

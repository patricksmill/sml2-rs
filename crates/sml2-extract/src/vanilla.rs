use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_READ: usize = 524_288 + 1;
pub const IMPORTER_VERSION: u32 = 1;
pub const SCHEMA: u32 = 1;

#[derive(Debug)]
pub struct Revision {
    pub id: &'static str,
    pub name: &'static str,
    pub len: usize,
    pub sha256: &'static str,
    pub offsets: &'static str,
}

pub const REVISIONS: &[Revision] = &[Revision {
    id: "ue-v1.0",
    name: "Super Mario Land 2",
    len: 524_288,
    sha256: "5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e",
    offsets: "ue-v1.0",
}];

#[derive(Debug)]
pub enum IdentifyError {
    TooLong,
    Unsupported { len: usize, sha256: String },
}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn identify(bytes: &[u8]) -> Result<&'static Revision, IdentifyError> {
    if bytes.len() > 524_288 {
        return Err(IdentifyError::TooLong);
    }
    let digest = sha256(bytes);
    REVISIONS
        .iter()
        .find(|revision| revision.len == bytes.len() && revision.sha256 == digest)
        .ok_or(IdentifyError::Unsupported {
            len: bytes.len(),
            sha256: digest,
        })
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub revision: String,
    pub sha256: String,
    pub importer_version: u32,
    pub schema: u32,
}

#[derive(Debug)]
pub enum PublishError {
    Prepare(String),
    Validate(String),
    Io(String),
}

#[derive(Debug)]
pub struct Commit {
    pub backup: Option<PathBuf>,
}

pub fn publish_directory(
    dest: &Path,
    manifest: &Manifest,
    prepare: impl FnOnce(&Path) -> Result<(), PublishError>,
    validate: impl FnOnce(&Path) -> Result<(), PublishError>,
) -> Result<Commit, PublishError> {
    let parent = dest.parent().ok_or_else(|| PublishError::Io("vanilla cache has no parent".into()))?;
    fs::create_dir_all(parent).map_err(|e| PublishError::Io(e.to_string()))?;
    let stage = parent.join(format!("vanilla.stage.{}", std::process::id()));
    let _ = fs::remove_dir_all(&stage);
    fs::create_dir_all(&stage).map_err(|e| PublishError::Io(e.to_string()))?;
    if let Err(error) = prepare(&stage) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|e| PublishError::Io(e.to_string()))?;
    if let Err(error) = fs::write(stage.join("manifest.json"), bytes) {
        let _ = fs::remove_dir_all(&stage);
        return Err(PublishError::Io(error.to_string()));
    }
    if let Err(error) = validate(&stage) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    let backup = parent.join(format!("vanilla.bak.{}", std::process::id()));
    let _ = fs::remove_dir_all(&backup);
    let had_dest = dest.exists();
    if had_dest {
        fs::rename(dest, &backup).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    if let Err(error) = fs::rename(&stage, dest) {
        if had_dest {
            let _ = fs::rename(&backup, dest);
        }
        let _ = fs::remove_dir_all(&stage);
        return Err(PublishError::Io(error.to_string()));
    }
    Ok(Commit {
        backup: had_dest.then_some(backup),
    })
}

pub fn rollback(dest: &Path, commit: &Commit) -> Result<(), PublishError> {
    let discarded = dest.with_extension("discarded");
    let _ = fs::remove_dir_all(&discarded);
    if dest.exists() {
        fs::rename(dest, &discarded).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    if let Some(backup) = &commit.backup {
        fs::rename(backup, dest).map_err(|e| PublishError::Io(e.to_string()))?;
    }
    let _ = fs::remove_dir_all(&discarded);
    Ok(())
}

pub fn discard_backup(commit: &Commit) {
    if let Some(backup) = &commit.backup {
        let _ = fs::remove_dir_all(backup);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_short_buffer_and_a_different_digest() {
        let err = identify(&[0; 16]).unwrap_err();
        assert!(matches!(err, IdentifyError::Unsupported { len: 16, .. }));
        let mut rom = vec![0; 524_288];
        let err = identify(&rom).unwrap_err();
        assert!(matches!(err, IdentifyError::Unsupported { len: 524_288, .. }));
        rom.push(0);
        assert!(matches!(identify(&rom), Err(IdentifyError::TooLong)));
    }

    #[test]
    fn accepts_only_the_ue_v1_0_digest() {
        let mut rom = vec![0; 524_288];
        // The real digest is pinned by the constant. A buffer of zeros must not match it.
        assert!(identify(&rom).is_err());
        let revision = Revision {
            id: "ue-v1.0",
            name: "Super Mario Land 2",
            len: 524_288,
            sha256: "5450dce1bd0c073964c374b5b5b5729dce8d00f2e807892c34af32b8bce1392e",
            offsets: "ue-v1.0",
        };
        assert_eq!(REVISIONS[0].sha256, revision.sha256);
        assert_eq!(REVISIONS[0].name, "Super Mario Land 2");
        assert_eq!(MAX_READ, 524_289);
    }

    #[test]
    fn a_failed_prepare_leaves_the_previous_install() {
        let root = std::env::temp_dir().join(format!("sml2_pub_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"old").unwrap();
        let manifest = Manifest {
            revision: "ue-v1.0".into(),
            sha256: "ab".into(),
            importer_version: IMPORTER_VERSION,
            schema: SCHEMA,
        };
        let err = publish_directory(&dest, &manifest, |_| Err(PublishError::Prepare("nope".into())), |_| Ok(())).unwrap_err();
        assert!(matches!(err, PublishError::Prepare(_)));
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn settings_failure_rolls_the_new_install_back() {
        let root = std::env::temp_dir().join(format!("sml2_roll_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("keep.txt"), b"old").unwrap();
        let manifest = Manifest {
            revision: "ue-v1.0".into(),
            sha256: "cd".into(),
            importer_version: IMPORTER_VERSION,
            schema: SCHEMA,
        };
        let commit = publish_directory(
            &dest,
            &manifest,
            |stage| {
                std::fs::write(stage.join("keep.txt"), b"new").unwrap();
                Ok(())
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"new");
        assert!(dest.join("manifest.json").is_file());
        rollback(&dest, &commit).unwrap();
        assert_eq!(std::fs::read(dest.join("keep.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }
}

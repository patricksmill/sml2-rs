use crate::settings::VanillaInstall;
use sml2_extract::vanilla::{
    IMPORTER_VERSION, IdentifyError, MAX_READ, Manifest, PublishError, SCHEMA, extract, identify,
    install_directory, sha256,
};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn vanilla_cache() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("", "", "sml2-rs")
        .map(|dirs| dirs.cache_dir().join("vanilla"))
        .ok_or_else(|| "Cannot find the vanilla cache directory".into())
}

pub fn read_rom(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_READ as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

/// Verify `rom` and make `dest` its install. Every supported ROM extracts to
/// the same files, so an install that already passes [`check_install`] for
/// this ROM is used as-is; only a missing, outdated or broken one is rebuilt.
pub fn install_rom(dest: &Path, rom: &[u8]) -> Result<VanillaInstall, String> {
    let revision = identify(rom).map_err(identify_error)?;
    let identity = VanillaInstall {
        revision: revision.id.to_string(),
        sha256: sha256(rom),
    };
    if check_install(dest, &identity).is_ok() {
        return Ok(identity);
    }
    let manifest = Manifest {
        revision: identity.revision.clone(),
        sha256: identity.sha256.clone(),
        importer_version: IMPORTER_VERSION,
        schema: SCHEMA,
    };
    install_directory(
        dest,
        &manifest,
        |stage| {
            let extracted = extract(rom, stage).map_err(PublishError::Prepare)?;
            if extracted.id != revision.id {
                return Err(PublishError::Prepare(
                    "extract revision did not match the identified ROM".into(),
                ));
            }
            // Keep the supplied original for optional IPS imports, including after restart.
            std::fs::write(stage.join(sml2_dx::SOURCE_ROM), rom)
                .map_err(|error| PublishError::Prepare(error.to_string()))?;
            Ok(())
        },
        validate_assets,
    )
    .map_err(publish_error)?;
    Ok(identity)
}

pub fn check_install(dir: &Path, expected: &VanillaInstall) -> Result<(), String> {
    let bytes = std::fs::read(dir.join("manifest.json")).map_err(|error| error.to_string())?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if manifest.importer_version != 1 || manifest.schema != 1 {
        return Err("This install was made by an older importer. Import the ROM again.".into());
    }
    if manifest.revision != expected.revision || manifest.sha256 != expected.sha256 {
        return Err("The vanilla install does not match the saved identity.".into());
    }
    validate_assets(dir).map_err(publish_error)
}

pub(crate) fn publish_error(error: PublishError) -> String {
    match error {
        PublishError::Prepare(message)
        | PublishError::Validate(message)
        | PublishError::Io(message) => message,
    }
}

pub(crate) fn identify_error(error: IdentifyError) -> String {
    match error {
        IdentifyError::TooLong => "ROM is longer than the allow-list".into(),
        IdentifyError::Unsupported { len, sha256 } => {
            format!("Unsupported vanilla ROM ({len} bytes, SHA-256 {sha256})")
        }
    }
}

fn validate_assets(stage: &Path) -> Result<(), PublishError> {
    let assets = sml2_runtime::content::SessionAssets::load_from(
        &sml2_runtime::content::ExtractedContent::new(stage),
    )
    .map_err(|e| PublishError::Validate(e.to_string()))?;
    for id in 0..32 {
        assets.level(id).map_err(PublishError::Validate)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_rom_stops_one_byte_past_the_allow_list() {
        let path = std::env::temp_dir().join(format!("sml2_rom_{}.bin", std::process::id()));
        std::fs::write(&path, vec![0; 524_289]).unwrap();
        let bytes = read_rom(&path).unwrap();
        assert_eq!(bytes.len(), 524_289);
        std::fs::remove_file(path).unwrap();
    }

    /// Run with SML2_ROM=/path/to/baserom.gb cargo test -p sml2-desktop
    /// an_install_is_reused -- --ignored
    #[test]
    #[ignore = "requires the user-supplied original ROM"]
    fn an_install_is_reused_and_only_a_broken_one_is_rebuilt() {
        let rom = read_rom(Path::new(
            &std::env::var_os("SML2_ROM").expect("set SML2_ROM"),
        ))
        .unwrap();
        let root = tempfile::tempdir().unwrap();
        let dest = root.path().join("vanilla");
        let identity = install_rom(&dest, &rom).unwrap();
        check_install(&dest, &identity).unwrap();
        // A matching install is used as-is: nothing in it is rewritten.
        std::fs::write(dest.join("marker.txt"), b"kept").unwrap();
        assert_eq!(install_rom(&dest, &rom).unwrap(), identity);
        assert_eq!(std::fs::read(dest.join("marker.txt")).unwrap(), b"kept");
        // A broken one is rebuilt whole.
        std::fs::write(dest.join("manifest.json"), b"{}").unwrap();
        assert_eq!(install_rom(&dest, &rom).unwrap(), identity);
        assert!(!dest.join("marker.txt").exists());
        check_install(&dest, &identity).unwrap();
    }

    #[test]
    fn an_unsupported_rom_leaves_the_install_alone() {
        let root = std::env::temp_dir().join(format!("sml2_install_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest = root.join("vanilla");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("marker.txt"), b"old").unwrap();
        let err = install_rom(&dest, &[0; 32]).unwrap_err();
        assert!(err.contains("Unsupported"));
        assert_eq!(std::fs::read(dest.join("marker.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_install_rejects_an_old_importer_and_a_different_identity() {
        let root = std::env::temp_dir().join(format!("sml2_check_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let expected = VanillaInstall {
            revision: "ue-v1.0".into(),
            sha256: "ab".into(),
        };
        std::fs::write(
            root.join("manifest.json"),
            r#"{"revision":"ue-v1.0","sha256":"ab","importer_version":0,"schema":1}"#,
        )
        .unwrap();
        assert_eq!(
            check_install(&root, &expected).unwrap_err(),
            "This install was made by an older importer. Import the ROM again."
        );
        std::fs::write(
            root.join("manifest.json"),
            r#"{"revision":"ue-v1.0","sha256":"ab","importer_version":1,"schema":2}"#,
        )
        .unwrap();
        assert_eq!(
            check_install(&root, &expected).unwrap_err(),
            "This install was made by an older importer. Import the ROM again."
        );
        std::fs::write(
            root.join("manifest.json"),
            r#"{"revision":"other","sha256":"cd","importer_version":1,"schema":1}"#,
        )
        .unwrap();
        std::fs::write(root.join("marker.txt"), b"old").unwrap();
        assert_eq!(
            check_install(&root, &expected).unwrap_err(),
            "The vanilla install does not match the saved identity."
        );
        assert_eq!(std::fs::read(root.join("marker.txt")).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(&root);
    }
}

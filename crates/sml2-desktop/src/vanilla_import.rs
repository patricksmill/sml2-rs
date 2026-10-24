use crate::settings::VanillaInstall;
use sml2_extract::vanilla::{
    extract, identify, publish_directory, sha256, IdentifyError, Manifest, PublishError,
    IMPORTER_VERSION, MAX_READ, SCHEMA,
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

pub fn install_rom(dest: &Path, rom: &[u8]) -> Result<sml2_extract::vanilla::Commit, String> {
    let revision = identify(rom).map_err(identify_error)?;
    let manifest = Manifest {
        revision: revision.id.to_string(),
        sha256: sha256(rom),
        importer_version: IMPORTER_VERSION,
        schema: SCHEMA,
    };
    publish_directory(
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
    .map_err(publish_error)
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

    #[test]
    fn a_bad_level_does_not_replace_the_install() {
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

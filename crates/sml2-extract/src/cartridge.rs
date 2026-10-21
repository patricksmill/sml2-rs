//! Cartridge metadata used for GBC boot-palette selection. Only $0134..=$014B
//! leaves the extractor; runtime palette selection never opens a ROM.

use std::{error::Error, fs, io, path::Path};

pub fn extract(rom: &[u8]) -> Result<[u8; 0x18], io::Error> {
    rom.get(0x134..0x14C)
        .and_then(|header| header.try_into().ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "cartridge header truncated at ROM $0134..$014B",
            )
        })
}

pub fn emit(rom: &[u8], extracted: &Path) -> Result<(), Box<dyn Error>> {
    let header = extract(rom)?;
    fs::create_dir_all(extracted)?;
    fs::write(extracted.join("cartridge_header.bin"), header)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_only_exact_header_metadata_and_rejects_truncation() {
        let dir = std::env::temp_dir().join(format!("sml2_cartridge_emit_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut rom = vec![0xAA; 0x150];
        let header = std::array::from_fn::<_, 24, _>(|i| i as u8);
        rom[0x134..0x14C].copy_from_slice(&header);
        emit(&rom, &dir).unwrap();
        assert_eq!(fs::read(dir.join("cartridge_header.bin")).unwrap(), header);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        assert!(emit(&rom[..0x14B], &dir).is_err());
        assert_eq!(fs::read(dir.join("cartridge_header.bin")).unwrap(), header);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn extracts_exact_header_range_and_rejects_truncation() {
        let mut rom = vec![0xAA; 0x150];
        let header = [
            0x4D, 0x41, 0x52, 0x49, 0x4F, 0x4C, 0x41, 0x4E, 0x44, 0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            3, 4, 2, 1, 1,
        ];
        rom[0x134..0x14C].copy_from_slice(&header);
        assert_eq!(extract(&rom).unwrap(), header);
        assert_eq!(
            extract(&rom[..0x14B]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let root = std::env::var_os("SML2_DECOMP_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../sml2decomp"));
        if let Ok(rom) = fs::read(root.join("baserom.gb")) {
            assert_eq!(extract(&rom).unwrap(), header);
        }
    }
}

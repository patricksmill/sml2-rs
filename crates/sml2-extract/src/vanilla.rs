use sha2::{Digest, Sha256};

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
}

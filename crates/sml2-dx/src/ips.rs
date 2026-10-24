//! Bounded IPS application. The resulting ROM still goes through exact DX identification.
use crate::{ImportError, ROM_LEN};

fn invalid(message: &str) -> ImportError {
    ImportError::InvalidPatch(message.into())
}

fn take<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8], ImportError> {
    if input.len() < count {
        return Err(invalid("the patch is incomplete"));
    }
    let (bytes, rest) = input.split_at(count);
    *input = rest;
    Ok(bytes)
}

fn address(bytes: &[u8]) -> usize {
    (usize::from(bytes[0]) << 16) | (usize::from(bytes[1]) << 8) | usize::from(bytes[2])
}

fn word(input: &mut &[u8]) -> Result<usize, ImportError> {
    let bytes = take(input, 2)?;
    Ok(usize::from(u16::from_be_bytes([bytes[0], bytes[1]])))
}

/// Apply literal and RLE records, zero-filled expansion, and optional EOF size.
/// Never modify the input ROM or allocate beyond the supported DX ROM size.
pub fn apply_ips(base: &[u8], mut patch: &[u8]) -> Result<Vec<u8>, ImportError> {
    if base.len() > ROM_LEN {
        return Err(invalid("the original ROM is too large"));
    }
    if take(&mut patch, 5)? != b"PATCH" {
        return Err(invalid("expected an IPS file starting with PATCH"));
    }
    let mut rom = base.to_vec();
    loop {
        let offset = take(&mut patch, 3)?;
        if offset == b"EOF" {
            match patch.len() {
                0 => {}
                3 => {
                    let size = address(patch);
                    if size > ROM_LEN {
                        return Err(invalid("the output exceeds the supported DX ROM size"));
                    }
                    rom.resize(size, 0);
                }
                _ => return Err(invalid("unexpected data after EOF")),
            }
            return Ok(rom);
        }
        let offset = address(offset);
        let length = word(&mut patch)?;
        let (length, repeated) = if length == 0 {
            let count = word(&mut patch)?;
            if count == 0 {
                return Err(invalid("an RLE record has zero length"));
            }
            (count, Some(take(&mut patch, 1)?[0]))
        } else {
            (length, None)
        };
        let end = offset
            .checked_add(length)
            .filter(|&end| end <= ROM_LEN)
            .ok_or_else(|| invalid("a record exceeds the supported DX ROM size"))?;
        rom.resize(rom.len().max(end), 0);
        if let Some(value) = repeated {
            rom[offset..end].fill(value);
        } else {
            rom[offset..end].copy_from_slice(take(&mut patch, length)?);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_rle_expansion_and_overlapping_records_preserve_the_original() {
        let base = [1, 2, 3, 4];
        let patch = b"PATCH\x00\x00\x01\x00\x02\xAA\xBB\x00\x00\x05\x00\x00\x00\x03\xCC\x00\x00\x02\x00\x01\xDD\x00\x00\x00\x00\x01\xEEEOF";
        assert_eq!(
            apply_ips(&base, patch).unwrap(),
            [0xEE, 0xAA, 0xDD, 4, 0, 0xCC, 0xCC, 0xCC]
        );
        assert_eq!(base, [1, 2, 3, 4]);
    }

    #[test]
    fn eof_size_can_shrink_or_expand_the_result() {
        assert_eq!(
            apply_ips(&[1, 2, 3], b"PATCHEOF\x00\x00\x02").unwrap(),
            [1, 2]
        );
        assert_eq!(
            apply_ips(&[1, 2], b"PATCHEOF\x00\x00\x04").unwrap(),
            [1, 2, 0, 0]
        );
    }

    #[test]
    fn incomplete_invalid_and_oversized_patches_are_rejected() {
        for patch in [
            &b""[..],
            &b"WRONGEOF"[..],
            &b"PATCH"[..],
            &b"PATCH\x00\x00\x00\x00\x02\x01"[..],
            &b"PATCH\x00\x00\x00\x00\x00\x00\x02"[..],
            &b"PATCH\x00\x00\x00\x00\x00\x00\x00\x01EOF"[..],
            &b"PATCH\x0F\xFF\xFF\x00\x02\x01\x02EOF"[..],
            &b"PATCHEOF\x10\x00\x01"[..],
            &b"PATCHEOFjunk"[..],
        ] {
            assert!(
                matches!(apply_ips(&[1, 2], patch), Err(ImportError::InvalidPatch(_))),
                "{patch:?}"
            );
        }
        assert!(apply_ips(&vec![0; ROM_LEN + 1], b"PATCHEOF").is_err());
        // A record ending exactly at the size limit remains valid.
        let rom = apply_ips(&[], b"PATCH\x0F\xFF\xFF\x00\x01\xABEOF").unwrap();
        assert_eq!(rom.len(), ROM_LEN);
        assert_eq!(rom[ROM_LEN - 1], 0xAB);
    }
}

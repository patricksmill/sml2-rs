//! Standard 16-bit PCM WAV audio encoder.
//!
//! Encodes stereo floating-point audio samples in `[-1.0, 1.0]` into standard
//! 16-bit little-endian PCM WAV files (44-byte RIFF header).

use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Encode interleaved stereo floating-point samples (`L, R, L, R...`) into a WAV byte vector.
pub fn encode_wav_pcm16(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let data_len = (num_samples * 2) as u32;
    let file_len = 36 + data_len;

    let mut buf = Vec::with_capacity(44 + num_samples * 2);

    // RIFF chunk
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_len.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt subchunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
    buf.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat (1 for PCM)
    buf.extend_from_slice(&2u16.to_le_bytes()); // NumChannels (2 for stereo)
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 2 * 2;
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&4u16.to_le_bytes()); // BlockAlign (channels * 2)
    buf.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample

    // data subchunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());

    for &s in samples {
        let pcm = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }

    buf
}

/// Write interleaved stereo floating-point samples to a WAV file at `path`.
pub fn write_wav_file(
    path: impl AsRef<Path>,
    samples: &[f32],
    sample_rate: u32,
) -> std::io::Result<()> {
    let bytes = encode_wav_pcm16(samples, sample_rate);
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_encoding() {
        let samples = [0.0f32, 0.5f32, -0.5f32, 1.0f32];
        let bytes = encode_wav_pcm16(&samples, 44100);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[12..16], b"fmt ");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(bytes.len(), 44 + samples.len() * 2);

        // Verify roundtrip sample conversion
        let pcm0 = i16::from_le_bytes([bytes[44], bytes[45]]);
        assert_eq!(pcm0, 0);
        let pcm1 = i16::from_le_bytes([bytes[46], bytes[47]]);
        assert!((pcm1 - 16383).abs() <= 1);
        let pcm2 = i16::from_le_bytes([bytes[48], bytes[49]]);
        assert!((pcm2 - -16383).abs() <= 1);
        let pcm3 = i16::from_le_bytes([bytes[50], bytes[51]]);
        assert_eq!(pcm3, 32767);
    }
}

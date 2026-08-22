use std::fmt;

pub const CORE_HEADER_BYTES: usize = 10;
pub const CORE_SYNC_WORD_BE: [u8; 4] = [0x7f, 0xfe, 0x80, 0x01];

/// Structural information for one complete 16-bit big-endian DTS core access unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreAccessUnit<'a> {
    pub data: &'a [u8],
    pub sample_rate: u32,
    pub channels: u8,
    pub samples: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreAccessUnitError {
    TooShort,
    UnsupportedSyncWord([u8; 4]),
    ReservedSampleRate(u8),
    UnsupportedChannelMode(u8),
    InvalidFrameSize(usize),
    Truncated { declared: usize, actual: usize },
}

impl fmt::Display for CoreAccessUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "DTS core access unit needs a ten-byte header"),
            Self::UnsupportedSyncWord(word) => write!(
                f,
                "unsupported DTS core sync word {:02x}{:02x}{:02x}{:02x}",
                word[0], word[1], word[2], word[3]
            ),
            Self::ReservedSampleRate(code) => {
                write!(f, "DTS core sample-rate code {code} is reserved")
            }
            Self::UnsupportedChannelMode(mode) => {
                write!(f, "DTS core channel-mode code {mode} is unsupported")
            }
            Self::InvalidFrameSize(size) => write!(f, "invalid DTS core frame size {size}"),
            Self::Truncated { declared, actual } => write!(
                f,
                "DTS core header declares {declared} bytes but only {actual} are present"
            ),
        }
    }
}

impl std::error::Error for CoreAccessUnitError {}

/// Parse one complete DTS core access unit without copying it.
///
/// Blu-ray and MPEG-TS carry the canonical 16-bit big-endian representation.
/// The byte-swapped and 14-bit packed DTS representations have different
/// physical frame sizes and are deliberately rejected instead of being
/// ambiguously interpreted.
pub fn parse_core_access_unit(data: &[u8]) -> Result<CoreAccessUnit<'_>, CoreAccessUnitError> {
    if data.len() < CORE_HEADER_BYTES {
        return Err(CoreAccessUnitError::TooShort);
    }
    let sync_word = [data[0], data[1], data[2], data[3]];
    if sync_word != CORE_SYNC_WORD_BE {
        return Err(CoreAccessUnitError::UnsupportedSyncWord(sync_word));
    }

    let blocks = u32::from((((u16::from(data[4]) << 8) | u16::from(data[5])) >> 2) & 0x7f) + 1;
    let frame_word = (u32::from(data[5]) << 16) | (u32::from(data[6]) << 8) | u32::from(data[7]);
    let frame_bytes = usize::try_from(((frame_word >> 4) & 0x3fff) + 1)
        .map_err(|_| CoreAccessUnitError::InvalidFrameSize(usize::MAX))?;
    if frame_bytes < CORE_HEADER_BYTES {
        return Err(CoreAccessUnitError::InvalidFrameSize(frame_bytes));
    }
    if frame_bytes > data.len() {
        return Err(CoreAccessUnitError::Truncated {
            declared: frame_bytes,
            actual: data.len(),
        });
    }

    let sample_rate_code = (data[8] >> 2) & 0x0f;
    let sample_rate = match sample_rate_code {
        1 => 8_000,
        2 => 16_000,
        3 => 32_000,
        6 => 11_025,
        7 => 22_050,
        8 => 44_100,
        11 => 12_000,
        12 => 24_000,
        13 => 48_000,
        14 => 96_000,
        15 => 192_000,
        code => return Err(CoreAccessUnitError::ReservedSampleRate(code)),
    };
    let channel_mode = ((data[7] & 0x0f) << 2) | (data[8] >> 6);
    let channels = match channel_mode {
        0 => 1,
        1..=4 => 2,
        5 | 6 => 3,
        7 | 8 => 4,
        9 => 5,
        10..=12 => 6,
        13 => 7,
        14 | 15 => 8,
        mode => return Err(CoreAccessUnitError::UnsupportedChannelMode(mode)),
    };

    Ok(CoreAccessUnit {
        data: &data[..frame_bytes],
        sample_rate,
        channels,
        samples: blocks * 32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_header(frame_bytes: usize, blocks: u8, channel_mode: u8, rate_code: u8) -> Vec<u8> {
        let mut data = vec![0; frame_bytes];
        data[..4].copy_from_slice(&CORE_SYNC_WORD_BE);
        let blocks_minus_one = u16::from(blocks - 1);
        data[4] |= (blocks_minus_one >> 6) as u8;
        data[5] |= ((blocks_minus_one & 0x3f) << 2) as u8;
        let size_minus_one = (frame_bytes - 1) as u32;
        data[5] |= ((size_minus_one >> 12) & 0x03) as u8;
        data[6] = (size_minus_one >> 4) as u8;
        data[7] = ((size_minus_one & 0x0f) << 4) as u8 | (channel_mode >> 2);
        data[8] = (channel_mode << 6) | (rate_code << 2);
        data
    }

    #[test]
    fn parses_complete_stereo_48k_core_frame() {
        let data = frame_header(1_024, 32, 2, 13);
        let unit = parse_core_access_unit(&data).expect("access unit");
        assert_eq!(unit.data.len(), 1_024);
        assert_eq!(unit.sample_rate, 48_000);
        assert_eq!(unit.channels, 2);
        assert_eq!(unit.samples, 1_024);
    }

    #[test]
    fn leaves_following_access_units_unconsumed() {
        let mut data = frame_header(512, 16, 9, 8);
        data.extend_from_slice(&frame_header(768, 32, 2, 13));
        let first = parse_core_access_unit(&data).expect("first access unit");
        assert_eq!(first.data.len(), 512);
        assert_eq!(first.sample_rate, 44_100);
        assert_eq!(first.channels, 5);
        assert_eq!(first.samples, 512);
        let second = parse_core_access_unit(&data[first.data.len()..]).expect("second access unit");
        assert_eq!(second.data.len(), 768);
    }

    #[test]
    fn rejects_bad_sync_and_truncation() {
        assert_eq!(
            parse_core_access_unit(&[0; 9]).unwrap_err(),
            CoreAccessUnitError::TooShort
        );
        let mut wrong_sync = frame_header(10, 1, 2, 13);
        wrong_sync[0] = 0xfe;
        assert!(matches!(
            parse_core_access_unit(&wrong_sync),
            Err(CoreAccessUnitError::UnsupportedSyncWord(_))
        ));
        let data = frame_header(1_024, 32, 2, 13);
        assert_eq!(
            parse_core_access_unit(&data[..512]).unwrap_err(),
            CoreAccessUnitError::Truncated {
                declared: 1_024,
                actual: 512
            }
        );
    }
}

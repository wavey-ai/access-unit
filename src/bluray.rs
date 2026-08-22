use std::fmt;

pub const LPCM_HEADER_BYTES: usize = 4;

/// Structural information for one Blu-ray LPCM access unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LpcmAccessUnit<'a> {
    /// The complete four-byte Blu-ray LPCM header.
    pub header: &'a [u8],
    /// Interleaved, big-endian PCM data following the header.
    pub payload: &'a [u8],
    pub sample_rate: u32,
    /// The number of source channels described by the channel-assignment code.
    pub channels: u8,
    /// The number of encoded channels, including Blu-ray padding channels.
    pub coded_channels: u8,
    pub bits_per_sample: u8,
    pub bytes_per_frame: usize,
    pub frames: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpcmAccessUnitError {
    TooShort,
    ReservedSampleRate(u8),
    ReservedChannelAssignment(u8),
    ReservedSampleDepth(u8),
    PayloadLengthMismatch {
        declared: usize,
        actual: usize,
    },
    UnalignedPayload {
        payload_bytes: usize,
        frame_bytes: usize,
    },
}

impl fmt::Display for LpcmAccessUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "Blu-ray LPCM access unit needs a four-byte header"),
            Self::ReservedSampleRate(code) => {
                write!(f, "Blu-ray LPCM sample-rate code {code} is reserved")
            }
            Self::ReservedChannelAssignment(code) => {
                write!(f, "Blu-ray LPCM channel-assignment code {code} is reserved")
            }
            Self::ReservedSampleDepth(code) => {
                write!(f, "Blu-ray LPCM sample-depth code {code} is reserved")
            }
            Self::PayloadLengthMismatch { declared, actual } => write!(
                f,
                "Blu-ray LPCM header declares {declared} payload bytes but {actual} are present"
            ),
            Self::UnalignedPayload {
                payload_bytes,
                frame_bytes,
            } => write!(
                f,
                "Blu-ray LPCM payload has {payload_bytes} bytes, which is not aligned to its {frame_bytes}-byte frames"
            ),
        }
    }
}

impl std::error::Error for LpcmAccessUnitError {}

/// Parse and validate one complete Blu-ray LPCM access unit without copying it.
pub fn parse_lpcm_access_unit(data: &[u8]) -> Result<LpcmAccessUnit<'_>, LpcmAccessUnitError> {
    if data.len() < LPCM_HEADER_BYTES {
        return Err(LpcmAccessUnitError::TooShort);
    }

    let declared = usize::from(u16::from_be_bytes([data[0], data[1]]));
    let payload = &data[LPCM_HEADER_BYTES..];
    if declared != payload.len() {
        return Err(LpcmAccessUnitError::PayloadLengthMismatch {
            declared,
            actual: payload.len(),
        });
    }

    let sample_rate_code = data[2] & 0x0f;
    let sample_rate = match sample_rate_code {
        1 => 48_000,
        4 => 96_000,
        5 => 192_000,
        code => return Err(LpcmAccessUnitError::ReservedSampleRate(code)),
    };
    let channel_assignment = data[2] >> 4;
    let channels = match channel_assignment {
        1 => 1,
        3 => 2,
        4 | 5 => 3,
        6 | 7 => 4,
        8 => 5,
        9 => 6,
        10 => 7,
        11 => 8,
        code => return Err(LpcmAccessUnitError::ReservedChannelAssignment(code)),
    };
    let coded_channels = if channels % 2 == 0 {
        channels
    } else {
        channels + 1
    };
    let sample_depth_code = data[3] >> 6;
    let bits_per_sample = match sample_depth_code {
        1 => 16,
        2 => 20,
        3 => 24,
        code => return Err(LpcmAccessUnitError::ReservedSampleDepth(code)),
    };
    let bytes_per_frame = match bits_per_sample {
        16 => usize::from(coded_channels) * 2,
        // Blu-ray packs two 20-bit samples into five bytes. The coded channel
        // count is always even, including its padding channel when needed.
        20 => usize::from(coded_channels) * 5 / 2,
        24 => usize::from(coded_channels) * 3,
        _ => unreachable!(),
    };
    if !payload.len().is_multiple_of(bytes_per_frame) {
        return Err(LpcmAccessUnitError::UnalignedPayload {
            payload_bytes: payload.len(),
            frame_bytes: bytes_per_frame,
        });
    }

    Ok(LpcmAccessUnit {
        header: &data[..LPCM_HEADER_BYTES],
        payload,
        sample_rate,
        channels,
        coded_channels,
        bits_per_sample,
        bytes_per_frame,
        frames: payload.len() / bytes_per_frame,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ffmpeg_stereo_48k_16_bit_access_unit() {
        let mut data = vec![0x03, 0xc0, 0x31, 0x40];
        data.resize(4 + 960, 0);
        let unit = parse_lpcm_access_unit(&data).expect("access unit");
        assert_eq!(unit.sample_rate, 48_000);
        assert_eq!(unit.channels, 2);
        assert_eq!(unit.coded_channels, 2);
        assert_eq!(unit.bits_per_sample, 16);
        assert_eq!(unit.bytes_per_frame, 4);
        assert_eq!(unit.frames, 240);
        assert_eq!(unit.payload.len(), 960);
    }

    #[test]
    fn accounts_for_odd_channel_padding_and_20_bit_packing() {
        let mut data = vec![0, 10, 0x41, 0x80];
        data.resize(14, 0);
        let unit = parse_lpcm_access_unit(&data).expect("access unit");
        assert_eq!(unit.channels, 3);
        assert_eq!(unit.coded_channels, 4);
        assert_eq!(unit.bits_per_sample, 20);
        assert_eq!(unit.bytes_per_frame, 10);
        assert_eq!(unit.frames, 1);
    }

    #[test]
    fn rejects_truncation_reserved_codes_and_bad_alignment() {
        assert_eq!(
            parse_lpcm_access_unit(&[0, 0, 0]).unwrap_err(),
            LpcmAccessUnitError::TooShort
        );
        assert_eq!(
            parse_lpcm_access_unit(&[0, 0, 0x30, 0x40]).unwrap_err(),
            LpcmAccessUnitError::ReservedSampleRate(0)
        );
        assert_eq!(
            parse_lpcm_access_unit(&[0, 4, 0x31, 0x40]).unwrap_err(),
            LpcmAccessUnitError::PayloadLengthMismatch {
                declared: 4,
                actual: 0
            }
        );
        assert_eq!(
            parse_lpcm_access_unit(&[0, 1, 0x31, 0x40, 0]).unwrap_err(),
            LpcmAccessUnitError::UnalignedPayload {
                payload_bytes: 1,
                frame_bytes: 4
            }
        );
    }
}

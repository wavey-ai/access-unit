#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpegVersion {
    V1,
    V2,
    V25,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpegLayer {
    LayerI,
    LayerII,
    LayerIII,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelMode {
    Stereo,
    JointStereo,
    DualChannel,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp3FrameHeader {
    pub version: MpegVersion,
    pub layer: MpegLayer,
    pub bitrate_kbps: u16,
    pub sample_rate: u32,
    pub padding: bool,
    pub channel_mode: ChannelMode,
    pub frame_length: usize,
    pub samples_per_frame: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3HeaderError {
    TooShort,
    InvalidSync,
    ReservedVersion,
    ReservedLayer,
    BadBitrate,
    BadSampleRate,
    ReservedEmphasis,
}

const BITRATE_V1_L1: [u16; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
const BITRATE_V1_L2: [u16; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];
const BITRATE_V1_L3: [u16; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const BITRATE_V2_L1: [u16; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
];
const BITRATE_V2_L2_L3: [u16; 14] = [
    8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160,
];

/// Returns true if a valid MP3 frame header is found anywhere in the slice.
pub fn is_mp3(data: &[u8]) -> bool {
    find_frame(data).is_some()
}

/// Scans for the first valid MP3 frame header and returns its offset and parsed header.
pub fn find_frame(data: &[u8]) -> Option<(usize, Mp3FrameHeader)> {
    if data.len() < 4 {
        return None;
    }

    for offset in 0..=data.len() - 4 {
        if let Ok(header) = parse_frame_header(&data[offset..]) {
            if header.frame_length >= 16 {
                return Some((offset, header));
            }
        }
    }

    None
}

pub fn parse_frame_header(input: &[u8]) -> Result<Mp3FrameHeader, Mp3HeaderError> {
    if input.len() < 4 {
        return Err(Mp3HeaderError::TooShort);
    }

    let b0 = input[0];
    let b1 = input[1];
    let b2 = input[2];
    let b3 = input[3];

    if b0 != 0xFF || (b1 & 0xE0) != 0xE0 {
        return Err(Mp3HeaderError::InvalidSync);
    }

    let version = match (b1 >> 3) & 0x03 {
        0b00 => MpegVersion::V25,
        0b10 => MpegVersion::V2,
        0b11 => MpegVersion::V1,
        _ => return Err(Mp3HeaderError::ReservedVersion),
    };

    let layer = match (b1 >> 1) & 0x03 {
        0b01 => MpegLayer::LayerIII,
        0b10 => MpegLayer::LayerII,
        0b11 => MpegLayer::LayerI,
        _ => return Err(Mp3HeaderError::ReservedLayer),
    };

    let bitrate_index = (b2 >> 4) & 0x0F;
    let bitrate_kbps =
        bitrate_kbps(version, layer, bitrate_index).ok_or(Mp3HeaderError::BadBitrate)?;

    let sample_rate_index = (b2 >> 2) & 0x03;
    let sample_rate =
        sample_rate(version, sample_rate_index).ok_or(Mp3HeaderError::BadSampleRate)?;

    let padding = ((b2 >> 1) & 0x01) == 1;

    let channel_mode = match (b3 >> 6) & 0x03 {
        0b00 => ChannelMode::Stereo,
        0b01 => ChannelMode::JointStereo,
        0b10 => ChannelMode::DualChannel,
        _ => ChannelMode::Mono,
    };

    let emphasis = b3 & 0x03;
    if emphasis == 0b10 {
        return Err(Mp3HeaderError::ReservedEmphasis);
    }

    let samples_per_frame = samples_per_frame(version, layer);
    let frame_length =
        frame_length_bytes(samples_per_frame, bitrate_kbps, sample_rate, layer, padding);

    Ok(Mp3FrameHeader {
        version,
        layer,
        bitrate_kbps,
        sample_rate,
        padding,
        channel_mode,
        frame_length,
        samples_per_frame,
    })
}

fn bitrate_kbps(version: MpegVersion, layer: MpegLayer, index: u8) -> Option<u16> {
    if index == 0 || index == 0x0F {
        return None;
    }

    let table = match (version, layer) {
        (MpegVersion::V1, MpegLayer::LayerI) => &BITRATE_V1_L1,
        (MpegVersion::V1, MpegLayer::LayerII) => &BITRATE_V1_L2,
        (MpegVersion::V1, MpegLayer::LayerIII) => &BITRATE_V1_L3,
        (_, MpegLayer::LayerI) => &BITRATE_V2_L1,
        _ => &BITRATE_V2_L2_L3,
    };

    Some(table[index as usize - 1])
}

fn sample_rate(version: MpegVersion, index: u8) -> Option<u32> {
    let value = match version {
        MpegVersion::V1 => match index {
            0 => 44_100,
            1 => 48_000,
            2 => 32_000,
            _ => 0,
        },
        MpegVersion::V2 => match index {
            0 => 22_050,
            1 => 24_000,
            2 => 16_000,
            _ => 0,
        },
        MpegVersion::V25 => match index {
            0 => 11_025,
            1 => 12_000,
            2 => 8_000,
            _ => 0,
        },
    };

    if value == 0 { None } else { Some(value) }
}

fn samples_per_frame(version: MpegVersion, layer: MpegLayer) -> u16 {
    match (layer, version) {
        (MpegLayer::LayerI, _) => 384,
        (MpegLayer::LayerII, _) => 1152,
        (MpegLayer::LayerIII, MpegVersion::V1) => 1152,
        (MpegLayer::LayerIII, _) => 576,
    }
}

fn frame_length_bytes(
    samples_per_frame: u16,
    bitrate_kbps: u16,
    sample_rate: u32,
    layer: MpegLayer,
    padding: bool,
) -> usize {
    let samples = samples_per_frame as u64;
    let bitrate = bitrate_kbps as u64 * 1000;
    let mut length = (samples * bitrate) / (sample_rate as u64 * 8);

    let padding_bytes = if layer == MpegLayer::LayerI && padding {
        4
    } else if padding {
        1
    } else {
        0
    };

    length += padding_bytes;
    length as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_header_bytes() -> [u8; 4] {
        // MPEG1 Layer III, 128 kbps, 44.1 kHz, stereo, no padding.
        [0xFF, 0xFB, 0x90, 0x00]
    }

    fn frame_bytes() -> Vec<u8> {
        let header = frame_header_bytes();
        let mut frame = header.to_vec();
        frame.resize(417, 0u8);
        frame
    }

    #[test]
    fn parses_mp3_header() {
        let header = parse_frame_header(&frame_header_bytes()).expect("header should parse");
        assert_eq!(header.version, MpegVersion::V1);
        assert_eq!(header.layer, MpegLayer::LayerIII);
        assert_eq!(header.bitrate_kbps, 128);
        assert_eq!(header.sample_rate, 44_100);
        assert_eq!(header.samples_per_frame, 1152);
        assert_eq!(header.frame_length, 417);
        assert_eq!(header.channel_mode, ChannelMode::Stereo);
        assert!(!header.padding);
    }

    #[test]
    fn detects_frame_in_stream() {
        let frame = frame_bytes();
        let mut stream = vec![0u8; 5];
        stream.extend_from_slice(&frame);
        stream.extend_from_slice(&frame);

        assert!(is_mp3(&stream));

        let (offset, header) = find_frame(&stream).expect("frame expected");
        assert_eq!(offset, 5);
        assert_eq!(header.frame_length, frame.len());
    }

    #[test]
    fn rejects_reserved_sample_rate() {
        let mut header = frame_header_bytes();
        header[2] |= 0x0C; // Set sample rate index to reserved value.
        assert!(matches!(
            parse_frame_header(&header),
            Err(Mp3HeaderError::BadSampleRate)
        ));
    }
}

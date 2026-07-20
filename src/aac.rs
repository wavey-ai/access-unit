use bytes::{Bytes, BytesMut};

/// Structural information for one ADTS AAC frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdtsFrameInfo {
    pub frame_length: usize,
    pub header_length: usize,
    pub sample_rate: u32,
    pub samples: u32,
}

/// Parse one ADTS frame header and verify that its complete frame is present.
pub fn parse_adts_frame(input: &[u8]) -> Option<AdtsFrameInfo> {
    if input.len() < 7 || input[0] != 0xff || input[1] & 0xf6 != 0xf0 {
        return None;
    }
    let sampling_frequency_index = (input[2] >> 2) & 0x0f;
    let sample_rate = adts_sample_rate(sampling_frequency_index)?;
    let header_length = if input[1] & 0x01 != 0 { 7 } else { 9 };
    if input.len() < header_length {
        return None;
    }
    let frame_length = (usize::from(input[3] & 0x03) << 11)
        | (usize::from(input[4]) << 3)
        | (usize::from(input[5]) >> 5);
    if frame_length < header_length || frame_length > input.len() {
        return None;
    }
    let raw_data_blocks = u32::from(input[6] & 0x03) + 1;
    Some(AdtsFrameInfo {
        frame_length,
        header_length,
        sample_rate,
        samples: 1_024 * raw_data_blocks,
    })
}

/// Split a buffer containing one or more complete ADTS frames without copying
/// their payload bytes. Returns `None` for truncation, garbage, or trailing
/// partial data.
pub fn split_adts_frames(data: Bytes) -> Option<Vec<Bytes>> {
    if data.is_empty() {
        return None;
    }
    let mut frames = Vec::new();
    let mut offset = 0usize;
    while offset < data.len() {
        let info = parse_adts_frame(&data[offset..])?;
        let end = offset.checked_add(info.frame_length)?;
        frames.push(data.slice(offset..end));
        offset = end;
    }
    Some(frames)
}

fn adts_sample_rate(index: u8) -> Option<u32> {
    Some(match index {
        0 => 96_000,
        1 => 88_200,
        2 => 64_000,
        3 => 48_000,
        4 => 44_100,
        5 => 32_000,
        6 => 24_000,
        7 => 22_050,
        8 => 16_000,
        9 => 12_000,
        10 => 11_025,
        11 => 8_000,
        12 => 7_350,
        _ => return None,
    })
}

pub fn is_aac(input: &[u8]) -> bool {
    // Check if we have at least 7 bytes (minimum ADTS header size)
    if input.len() < 7 {
        return false;
    }

    // Check for the ADTS sync word (12 bits)
    if input[0] != 0xFF || (input[1] & 0xF0) != 0xF0 {
        return false;
    }

    let layer = (input[1] & 0x06) >> 1;
    if layer != 0 {
        // Layer must be '00' for AAC
        return false;
    }

    // Check profile (2 bits)
    let profile = (input[2] & 0xC0) >> 6;
    if profile == 3 {
        // '11' is reserved
        return false;
    }

    // Check sampling frequency index (4 bits)
    let sampling_freq_index = (input[2] & 0x3C) >> 2;
    if sampling_freq_index > 11 {
        // Valid range is 0-11
        return false;
    }

    // All checks passed
    true
}

pub fn extract_aac_data(sound_data: &Bytes) -> Option<Bytes> {
    if sound_data.len() < 7 {
        return None;
    }

    // Check for the ADTS sync word
    if sound_data[0] != 0xFF || (sound_data[1] & 0xF0) != 0xF0 {
        return None;
    }

    // Parse the ADTS header
    let protection_absent: bool = (sound_data[1] & 0x01) == 0x01;
    let header_size: usize = if protection_absent { 7 } else { 9 };

    if sound_data.len() < header_size {
        return None;
    }

    let frame_length: usize = (((sound_data[3] as usize & 0x03) << 11)
        | ((sound_data[4] as usize) << 3)
        | ((sound_data[5] as usize) >> 5)) as usize;

    if sound_data.len() < frame_length {
        return None;
    }

    Some(sound_data.slice(header_size..frame_length))
}

pub fn ensure_adts_header(data: Bytes, channels: u8, sample_rate: u32) -> Bytes {
    // Assume that the first byte might contain the ASC if `extract_aac_data` finds no ADTS header
    if extract_aac_data(&data).is_none() {
        // Assuming data[0] is present and is the first byte of ASC
        // Parse the profile from the ASC
        let audio_object_type = data[0] >> 3; // First 5 bits contain the audio object type
        let profile = match audio_object_type {
            1 => 0x66, // AAC-LC
            2 => 0x67, // HE-AAC v1
            5 => 0x68, // HE-AAC v2
            _ => 0x66, // Default to AAC-LC if unknown
        };

        let header = create_adts_header(profile, channels, sample_rate, data.len() - 2, false);
        let mut payload = BytesMut::from(&header[..]);
        payload.extend_from_slice(&data[2..]); // Skip the first two bytes if they are part of ASC

        return payload.freeze();
    }

    return data;
}

pub fn create_adts_header(
    codec_id: u8,
    channels: u8,
    sample_rate: u32,
    aac_frame_length: usize,
    has_crc: bool,
) -> Vec<u8> {
    let profile_object_type = match codec_id {
        0x66 => 1, // AAC LC (internally set as `1`, should directly be `01` in bits)
        0x67 => 2, // AAC HEV1
        0x68 => 3, // AAC HEV2
        _ => 1,    // Default to AAC LC
    };

    let sample_rate_index = sample_rate_index(sample_rate);
    let channel_config = channels.min(7);
    let header_length = if has_crc { 9 } else { 7 };
    let frame_length = aac_frame_length + header_length;

    let mut header = Vec::with_capacity(header_length);
    let protection_absent = if has_crc { 0 } else { 1 };

    header.push(0xFF);
    header.push(0xF0 | protection_absent);

    let profile_and_sampling =
        (profile_object_type << 6) | (sample_rate_index << 2) | (channel_config >> 2);
    header.push(profile_and_sampling);

    let frame_length_high = ((frame_length >> 11) & 0x03) as u8;
    let frame_length_mid = ((frame_length >> 3) & 0xFF) as u8;
    header.push((channel_config & 3) << 6 | frame_length_high);
    header.push(frame_length_mid);

    let frame_length_low = ((frame_length & 0x07) << 5) | 0x1F;
    header.push(frame_length_low as u8);
    header.push(0xFC);

    if has_crc {
        header.extend_from_slice(&[0x00, 0x00]);
    }

    header
}

fn sample_rate_index(sample_rate: u32) -> u8 {
    match sample_rate {
        96000 => 0x0,
        88200 => 0x1,
        64000 => 0x2,
        48000 => 0x3,
        44100 => 0x4,
        32000 => 0x5,
        24000 => 0x6,
        22050 => 0x7,
        16000 => 0x8,
        12000 => 0x9,
        11025 => 0xA,
        8000 => 0xB,
        7350 => 0xC,
        _ => 0xF, // Invalid sample rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_splits_multiple_complete_adts_frames() {
        let first_payload = vec![0x11; 31];
        let second_payload = vec![0x22; 47];
        let mut encoded = create_adts_header(0x66, 2, 48_000, first_payload.len(), false);
        encoded.extend_from_slice(&first_payload);
        encoded.extend_from_slice(&create_adts_header(
            0x66,
            2,
            48_000,
            second_payload.len(),
            false,
        ));
        encoded.extend_from_slice(&second_payload);

        let frames = split_adts_frames(Bytes::from(encoded)).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(parse_adts_frame(&frames[0]).unwrap().sample_rate, 48_000);
        assert_eq!(parse_adts_frame(&frames[0]).unwrap().samples, 1_024);
        assert_eq!(frames[0].len(), 7 + first_payload.len());
        assert_eq!(frames[1].len(), 7 + second_payload.len());
    }

    #[test]
    fn rejects_partial_or_trailing_adts_data() {
        let mut frame = create_adts_header(0x66, 2, 48_000, 8, false);
        frame.extend_from_slice(&[0; 7]);
        assert!(split_adts_frames(Bytes::from(frame)).is_none());

        let mut frame = create_adts_header(0x66, 2, 48_000, 1, false);
        frame.push(0xaa);
        frame.push(0xff);
        assert!(split_adts_frames(Bytes::from(frame)).is_none());
    }
}

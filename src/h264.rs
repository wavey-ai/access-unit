/// Byte framing used for one H.264 access unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
    /// ISO BMFF/AVCC length-prefixed NAL units.
    Avcc,
    /// Annex-B NAL units separated by start codes.
    AnnexB,
}

/// Detect H.264 framing without mistaking a valid AVCC length for an Annex-B
/// start code.
///
/// AVCC is validated first because a four-byte length such as `00 00 01 10`
/// is both a valid 272-byte NAL-unit length and superficially start-code-like.
pub fn detect_framing(data: &[u8], avcc_length_size: usize) -> Option<Framing> {
    if is_valid_avcc_sample(data, avcc_length_size) {
        Some(Framing::Avcc)
    } else if is_valid_annex_b_sample(data) {
        Some(Framing::AnnexB)
    } else {
        None
    }
}

/// Return whether `data` is one complete AVCC length-prefixed H.264 sample.
pub fn is_valid_avcc_sample(mut data: &[u8], length_size: usize) -> bool {
    if !(1..=4).contains(&length_size) || data.is_empty() {
        return false;
    }

    let mut nalus = 0usize;
    while !data.is_empty() {
        if data.len() < length_size {
            return false;
        }
        let mut nalu_len = 0usize;
        for byte in &data[..length_size] {
            nalu_len = (nalu_len << 8) | usize::from(*byte);
        }
        data = &data[length_size..];
        if nalu_len == 0 || nalu_len > data.len() {
            return false;
        }
        let nalu = &data[..nalu_len];
        if !has_valid_nalu_header(nalu) {
            return false;
        }
        data = &data[nalu_len..];
        nalus += 1;
    }
    nalus > 0
}

/// Return whether `data` is a complete Annex-B H.264 sample containing at
/// least one structurally valid NAL unit.
pub fn is_valid_annex_b_sample(data: &[u8]) -> bool {
    let Some((first_start, first_len)) = find_start_code(data, 0) else {
        return false;
    };
    if data[..first_start].iter().any(|byte| *byte != 0) {
        return false;
    }

    let mut nalu_start = first_start + first_len;
    let mut nalus = 0usize;
    loop {
        let next = find_start_code(data, nalu_start);
        let mut nalu_end = next.map_or(data.len(), |(start, _)| start);
        while nalu_end > nalu_start && data[nalu_end - 1] == 0 {
            nalu_end -= 1;
        }
        if !has_valid_nalu_header(&data[nalu_start..nalu_end]) {
            return false;
        }
        nalus += 1;

        let Some((start, start_len)) = next else {
            break;
        };
        nalu_start = start + start_len;
    }
    nalus > 0
}

fn has_valid_nalu_header(nalu: &[u8]) -> bool {
    let Some(header) = nalu.first() else {
        return false;
    };
    let nalu_type = header & 0x1f;
    header & 0x80 == 0 && (1..=23).contains(&nalu_type)
}

fn find_start_code(data: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut offset = from;
    while offset + 2 < data.len() {
        if data[offset] == 0 && data[offset + 1] == 0 {
            let mut one = offset + 2;
            while one < data.len() && data[one] == 0 {
                one += 1;
            }
            if one < data.len() && data[one] == 1 {
                return Some((offset, one - offset + 1));
            }
        }
        offset += 1;
    }
    None
}

pub fn is_nalu(data: &[u8]) -> bool {
    if data.len() < 3 {
        return false;
    }

    data.windows(3).any(|window| match window {
        [0x00, 0x00, 0x01] => true,
        [0x00, 0x00, 0x00] if data.len() >= 4 && data[3] == 0x01 => true,
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nalu_detection() {
        // Start codes at beginning
        assert!(is_nalu(&[0x00, 0x00, 0x01, 0x09, 0xFF]));
        assert!(is_nalu(&[0x00, 0x00, 0x00, 0x01, 0x09, 0xFF]));

        // Start codes in middle
        assert!(is_nalu(&[0xFF, 0xFF, 0x00, 0x00, 0x01, 0x09]));
        assert!(is_nalu(&[0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01, 0x09]));

        // Start codes at end
        assert!(is_nalu(&[0xFF, 0xFF, 0x00, 0x00, 0x01]));
        assert!(is_nalu(&[0xFF, 0xFF, 0x00, 0x00, 0x00, 0x01]));

        // Multiple start codes
        assert!(is_nalu(&[
            0xFF, 0x00, 0x00, 0x01, 0x09, 0x00, 0x00, 0x00, 0x01
        ]));

        // Invalid data
        assert!(!is_nalu(&[0x00, 0x01, 0x00, 0x01]));
        assert!(!is_nalu(&[0xFF, 0xFF, 0xFF, 0xFF]));

        // Too short
        assert!(!is_nalu(&[0x00, 0x00]));
        assert!(!is_nalu(&[]));

        // Partial start code at end
        assert!(!is_nalu(&[0xFF, 0x00, 0x00]));
        assert!(!is_nalu(&[0xFF, 0x00, 0x00, 0x00]));
    }

    #[test]
    fn detects_avcc_before_an_ambiguous_start_code_prefix() {
        let mut nalu = vec![0x55; 0x110];
        nalu[0] = 0x41;
        let mut sample = Vec::with_capacity(4 + nalu.len());
        sample.extend_from_slice(&(nalu.len() as u32).to_be_bytes());
        sample.extend_from_slice(&nalu);

        assert!(sample.starts_with(&[0, 0, 1]));
        assert!(is_valid_avcc_sample(&sample, 4));
        assert_eq!(detect_framing(&sample, 4), Some(Framing::Avcc));
    }

    #[test]
    fn detects_annex_b_with_three_and_four_byte_start_codes() {
        let sample = [0, 0, 0, 1, 0x09, 0xf0, 0, 0, 1, 0x67, 0x42, 0xc0, 0x1f];
        assert!(is_valid_annex_b_sample(&sample));
        assert!(!is_valid_avcc_sample(&sample, 4));
        assert_eq!(detect_framing(&sample, 4), Some(Framing::AnnexB));
    }

    #[test]
    fn rejects_truncated_or_invalid_h264_samples() {
        assert_eq!(detect_framing(&[0, 0, 0, 8, 0x41, 1, 2], 4), None);
        assert!(!is_valid_avcc_sample(&[0, 0, 0, 1, 0x80], 4));
        assert!(!is_valid_annex_b_sample(&[0, 0, 1, 0x80]));
        assert!(!is_valid_avcc_sample(&[0, 0, 0, 1, 0x41], 0));
    }
}

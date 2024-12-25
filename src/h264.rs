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
}

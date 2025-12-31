/// EBML header magic bytes for WebM/Matroska
const EBML_MAGIC: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];

/// Check for WebM container format.
/// WebM uses EBML with magic bytes 0x1A 0x45 0xDF 0xA3 and DocType "webm".
pub fn is_webm(data: &[u8]) -> bool {
    if data.len() < 4 || data[0..4] != EBML_MAGIC {
        return false;
    }
    // Look for "webm" DocType in the first 64 bytes
    let search_len = data.len().min(64);
    data[..search_len].windows(4).any(|w| w == b"webm")
}

/// Check for Matroska container format (MKV).
/// Matroska uses EBML with magic bytes 0x1A 0x45 0xDF 0xA3 and DocType "matroska".
pub fn is_matroska(data: &[u8]) -> bool {
    if data.len() < 4 || data[0..4] != EBML_MAGIC {
        return false;
    }
    // Look for "matroska" DocType in the first 64 bytes
    let search_len = data.len().min(64);
    data[..search_len].windows(8).any(|w| w == b"matroska")
}

/// Check for any EBML-based container (WebM or Matroska).
pub fn is_ebml(data: &[u8]) -> bool {
    data.len() >= 4 && data[0..4] == EBML_MAGIC
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webm_detection() {
        // Valid WebM header (EBML magic + some content with "webm" doctype)
        let webm_data = [
            0x1A, 0x45, 0xDF, 0xA3, // EBML magic
            0x01, 0x00, 0x00, 0x00, // size
            0x00, 0x00, 0x1F, 0x43, // some data
            b'w', b'e', b'b', b'm', // doctype
        ];
        assert!(is_webm(&webm_data));
        assert!(is_ebml(&webm_data));
        assert!(!is_matroska(&webm_data));
    }

    #[test]
    fn test_matroska_detection() {
        // Valid Matroska header
        let mkv_data = [
            0x1A, 0x45, 0xDF, 0xA3, // EBML magic
            0x01, 0x00, 0x00, 0x00, // size
            b'm', b'a', b't', b'r', b'o', b's', b'k', b'a', // doctype
        ];
        assert!(!is_webm(&mkv_data));
        assert!(is_ebml(&mkv_data));
        assert!(is_matroska(&mkv_data));
    }

    #[test]
    fn test_invalid_data() {
        assert!(!is_webm(&[]));
        assert!(!is_webm(&[0x00, 0x00, 0x00]));
        assert!(!is_webm(b"RIFF"));
        assert!(!is_ebml(&[]));
    }
}

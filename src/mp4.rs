use crate::AudioType;

/// Returns true if the data starts with a valid MP4 `ftyp` box.
pub fn is_mp4(data: &[u8]) -> bool {
    matches!(next_box(data, 0), Some((name, _, _)) if &name == b"ftyp")
}

/// Attempts to find the first audio track in the MP4 and map its sample entry to an `AudioType`.
pub fn detect_audio_track(data: &[u8]) -> Option<AudioType> {
    if !is_mp4(data) {
        return None;
    }

    let moov = find_child(data, *b"moov")?;

    let mut offset = 0;
    while let Some((name, trak, next_offset)) = next_box(moov, offset) {
        if &name == b"trak" {
            if let Some(audio_type) = parse_trak(trak) {
                return Some(audio_type);
            }
        }
        offset = next_offset;
    }

    None
}

fn parse_trak(trak: &[u8]) -> Option<AudioType> {
    let mdia = find_child(trak, *b"mdia")?;
    if !is_audio_handler(mdia) {
        return None;
    }

    let minf = find_child(mdia, *b"minf")?;
    let stbl = find_child(minf, *b"stbl")?;
    let stsd = find_child(stbl, *b"stsd")?;

    parse_stsd(stsd)
}

fn is_audio_handler(mdia: &[u8]) -> bool {
    let hdlr = match find_child(mdia, *b"hdlr") {
        Some(hdlr) => hdlr,
        None => return false,
    };

    if hdlr.len() < 12 {
        return false;
    }

    // hdlr full box: version/flags (4), pre_defined (4), handler_type (4)
    &hdlr[8..12] == b"soun"
}

fn parse_stsd(stsd: &[u8]) -> Option<AudioType> {
    if stsd.len() < 8 {
        return None;
    }

    let entry_count = u32::from_be_bytes(stsd[4..8].try_into().ok()?) as usize;
    let mut offset = 8;

    for _ in 0..entry_count {
        let (format, next_offset) = parse_stsd_entry(stsd, offset)?;
        let audio_type = fourcc_to_audio_type(format);
        if audio_type != AudioType::Unknown {
            return Some(audio_type);
        }
        offset = next_offset;
    }

    None
}

fn parse_stsd_entry(stsd: &[u8], offset: usize) -> Option<([u8; 4], usize)> {
    if offset + 8 > stsd.len() {
        return None;
    }

    let size = u32::from_be_bytes(stsd[offset..offset + 4].try_into().ok()?) as usize;
    if size < 8 || offset + size > stsd.len() {
        return None;
    }

    let mut format = [0u8; 4];
    format.copy_from_slice(&stsd[offset + 4..offset + 8]);

    Some((format, offset + size))
}

fn find_child<'a>(data: &'a [u8], target: [u8; 4]) -> Option<&'a [u8]> {
    let mut offset = 0;
    while let Some((name, content, next_offset)) = next_box(data, offset) {
        if name == target {
            return Some(content);
        }
        offset = next_offset;
    }
    None
}

fn next_box<'a>(data: &'a [u8], offset: usize) -> Option<([u8; 4], &'a [u8], usize)> {
    if offset + 8 > data.len() {
        return None;
    }

    let size32 = u32::from_be_bytes(data[offset..offset + 4].try_into().ok()?);
    let mut header_len = 8usize;
    let mut size = size32 as u64;

    if size32 == 1 {
        if offset + 16 > data.len() {
            return None;
        }
        size = u64::from_be_bytes(data[offset + 8..offset + 16].try_into().ok()?);
        header_len = 16;
    } else if size32 == 0 {
        size = (data.len() - offset) as u64;
    }

    if size < header_len as u64 {
        return None;
    }

    let end = offset.checked_add(size as usize)?;
    if end > data.len() {
        return None;
    }

    let mut name = [0u8; 4];
    name.copy_from_slice(&data[offset + 4..offset + 8]);

    let content_start = offset + header_len;
    Some((name, &data[content_start..end], end))
}

fn fourcc_to_audio_type(code: [u8; 4]) -> AudioType {
    match &code {
        b"mp4a" => AudioType::AAC,
        b"fLaC" | b"FLAC" => AudioType::FLAC,
        b"Opus" | b"opus" => AudioType::Opus,
        b"mp3 " | b".mp3" => AudioType::MP3,
        _ => AudioType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn read(path: &str) -> Vec<u8> {
        fs::read(path).unwrap_or_else(|err| panic!("read {}: {}", path, err))
    }

    #[test]
    fn detects_mp4_container() {
        let data = read("testdata/mp4/heat.mp4");
        assert!(is_mp4(&data));
    }

    #[test]
    fn extracts_audio_type() {
        let data = read("testdata/mp4/heat.mp4");
        assert_eq!(detect_audio_track(&data), Some(AudioType::AAC));
    }
}

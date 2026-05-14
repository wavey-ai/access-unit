use bytes::Bytes;

pub mod aac;
pub mod chunk;
pub mod flac;
pub mod h264;
pub mod mp3;
pub mod mp4;
pub mod webm;

pub const PSI_STREAM_MP3: u8 = 0x04; // ISO/IEC 13818-3 Audio
pub const PSI_STREAM_PRIVATE_DATA: u8 = 0x06;
pub const PSI_STREAM_H264: u8 = 0x1b; // H.264
pub const PSI_STREAM_AAC: u8 = 0x0f;
pub const PSI_STREAM_MPEG4_AAC: u8 = 0x1c;
pub const PSI_STREAM_AUDIO_OPUS: u8 = 0x9c;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioType {
    Unknown,
    AAC, // Raw AAC (ADTS format)
    M4A, // AAC in MP4/M4A container
    FLAC,
    MP3,
    OggOpus,
    OggVorbis,
    OggSpeex,
    Opus,
    Wav,
    WebM,
    ALAC,
    AIFF,
    AC3,
}

#[derive(Debug, Clone)]
pub struct Fmp4 {
    pub init: Option<Bytes>,
    pub key: bool,
    pub data: Bytes,
    pub duration: u32,
}

#[derive(Debug, Clone)]
pub struct AccessUnit {
    pub key: bool,
    pub pts: u64,
    pub dts: u64,
    pub data: Bytes,
    pub stream_type: u8,
    pub id: u64,
}

pub fn detect_audio(data: &[u8]) -> AudioType {
    // Parse MP4 sample entries before falling back to raw/container signatures.
    // A bare ftyp/M4A brand is not enough to know whether the audio is AAC,
    // ALAC, or another codec.
    if let Some(audio_type) = mp4::detect_audio_track(data) {
        return audio_type;
    }
    if is_caf_alac(data) {
        AudioType::ALAC
    } else if flac::is_flac(data) {
        AudioType::FLAC
    } else if aac::is_aac(data) {
        AudioType::AAC
    } else if webm::is_webm(data) {
        AudioType::WebM
    } else if is_ogg_opus(data) {
        AudioType::OggOpus
    } else if is_ogg_vorbis(data) {
        AudioType::OggVorbis
    } else if is_ogg_speex(data) {
        AudioType::OggSpeex
    } else if is_opus(data) {
        AudioType::Opus
    } else if is_aiff(data) {
        AudioType::AIFF
    } else if is_wav(data) {
        AudioType::Wav
    } else if is_ac3(data) {
        AudioType::AC3
    } else if mp3::is_mp3(data) {
        AudioType::MP3
    } else {
        AudioType::Unknown
    }
}

pub fn is_mp4(data: &[u8]) -> bool {
    mp4::is_mp4(data)
}

pub fn is_webm(data: &[u8]) -> bool {
    webm::is_webm(data)
}

fn is_wav(data: &[u8]) -> bool {
    data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE"
}

pub fn is_aiff(data: &[u8]) -> bool {
    data.len() >= 12
        && &data[0..4] == b"FORM"
        && (&data[8..12] == b"AIFF" || &data[8..12] == b"AIFC")
}

pub fn is_caf_alac(data: &[u8]) -> bool {
    data.starts_with(b"caff") && data.windows(b"alac".len()).any(|w| w == b"alac")
}

pub fn is_ac3(data: &[u8]) -> bool {
    if data.len() < 5 || data[0] != 0x0b || data[1] != 0x77 {
        return false;
    }

    let fscod = data[4] >> 6;
    let frmsizecod = data[4] & 0x3f;
    fscod < 3 && frmsizecod <= 37
}

fn is_opus(data: &[u8]) -> bool {
    if data.starts_with(b"OggS") {
        return false;
    }

    let search_len = data.len().min(64);
    data[..search_len]
        .windows(b"OpusHead".len())
        .any(|w| w == b"OpusHead")
}

fn is_ogg_opus(data: &[u8]) -> bool {
    if data.len() < 36 || !data.starts_with(b"OggS") {
        return false;
    }
    // Look for the Opus ID header within the first page payload
    let search_len = data.len().min(256);
    data[..search_len]
        .windows(b"OpusHead".len())
        .any(|w| w == b"OpusHead")
}

pub fn is_ogg_vorbis(data: &[u8]) -> bool {
    if !data.starts_with(b"OggS") {
        return false;
    }

    let search_len = data.len().min(256);
    data[..search_len]
        .windows(b"\x01vorbis".len())
        .any(|w| w == b"\x01vorbis")
}

pub fn is_ogg_speex(data: &[u8]) -> bool {
    if !data.starts_with(b"OggS") {
        return false;
    }

    let search_len = data.len().min(256);
    data[..search_len]
        .windows(b"Speex   ".len())
        .any(|w| w == b"Speex   ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn read(path: &str) -> Vec<u8> {
        fs::read(path).unwrap_or_else(|err| panic!("read {}: {}", path, err))
    }

    #[test]
    fn detect_aac_from_testdata() {
        let data = read("testdata/wav_stereo/A_Tusk_is_used_to_make_costly_gifts.wav.aac");
        assert_eq!(detect_audio(&data), AudioType::AAC);
    }

    #[test]
    fn detect_flac_from_testdata() {
        let data = read("testdata/flac/A_Tusk_is_used_to_make_costly_gifts.flac");
        assert_eq!(detect_audio(&data), AudioType::FLAC);
    }

    #[test]
    fn detect_mp3_from_testdata() {
        let data = read("testdata/mp3/A_Tusk_is_used_to_make_costly_gifts.mp3");
        assert_eq!(detect_audio(&data), AudioType::MP3);

        let (offset, header) = mp3::find_frame(&data).expect("mp3 frame");
        assert!(offset < data.len());
        assert!(header.frame_length > 0);
    }

    #[test]
    fn detect_mp4_audio_from_testdata() {
        let data = read("testdata/mp4/heat.mp4");
        assert_eq!(detect_audio(&data), AudioType::M4A);
    }

    #[test]
    fn detect_wav_from_testdata() {
        let data = read("testdata/wav_stereo/A_Tusk_is_used_to_make_costly_gifts.wav");
        assert_eq!(detect_audio(&data), AudioType::Wav);
    }

    #[test]
    fn detect_opus_from_testdata() {
        let data = read("testdata/opus/A_Tusk_is_used_to_make_costly_gifts.opus");
        assert_eq!(detect_audio(&data), AudioType::Opus);
    }

    #[test]
    fn detect_ogg_opus_from_testdata() {
        let data = read("testdata/ogg_opus/A_Tusk_is_used_to_make_costly_gifts.ogg");
        assert_eq!(detect_audio(&data), AudioType::OggOpus);
    }

    #[test]
    fn detect_webm_from_testdata() {
        let data = read("testdata/webm/A_Tusk_is_used_to_make_costly_gifts.webm");
        assert_eq!(detect_audio(&data), AudioType::WebM);
    }

    fn ogg_with_marker(marker: &[u8]) -> Vec<u8> {
        let mut data = b"OggS".to_vec();
        data.resize(64, 0);
        data.extend_from_slice(marker);
        data
    }

    #[test]
    fn detect_ogg_vorbis_signature() {
        let data = ogg_with_marker(b"\x01vorbis");
        assert_eq!(detect_audio(&data), AudioType::OggVorbis);
    }

    #[test]
    fn detect_ogg_speex_signature() {
        let data = ogg_with_marker(b"Speex   ");
        assert_eq!(detect_audio(&data), AudioType::OggSpeex);
    }

    #[test]
    fn detect_caf_alac_signature() {
        let mut data = b"caff".to_vec();
        data.resize(32, 0);
        data.extend_from_slice(b"alac");
        assert_eq!(detect_audio(&data), AudioType::ALAC);
    }

    #[test]
    fn detect_aiff_signatures() {
        let mut aiff = b"FORM\0\0\0\x04AIFF".to_vec();
        assert_eq!(detect_audio(&aiff), AudioType::AIFF);

        aiff[8..12].copy_from_slice(b"AIFC");
        assert_eq!(detect_audio(&aiff), AudioType::AIFF);
    }

    #[test]
    fn detect_raw_ac3_syncframe() {
        let data = [0x0b, 0x77, 0x00, 0x00, 0x00];
        assert_eq!(detect_audio(&data), AudioType::AC3);
    }
}

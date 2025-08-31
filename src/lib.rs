use bytes::Bytes;

pub mod aac;
pub mod chunk;
pub mod flac;
pub mod h264;

pub const PSI_STREAM_MP3: u8 = 0x04; // ISO/IEC 13818-3 Audio
pub const PSI_STREAM_PRIVATE_DATA: u8 = 0x06;
pub const PSI_STREAM_H264: u8 = 0x1b; // H.264
pub const PSI_STREAM_AAC: u8 = 0x0f;
pub const PSI_STREAM_MPEG4_AAC: u8 = 0x1c;
pub const PSI_STREAM_AUDIO_OPUS: u8 = 0x9c;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioType {
    Unknown,
    AAC,
    FLAC,
    Opus,
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
    if flac::is_flac(data) {
        AudioType::FLAC
    } else if aac::is_aac(data) {
        AudioType::AAC
    } else {
        AudioType::Unknown
    }
}

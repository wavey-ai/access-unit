use bytes::Bytes;

pub mod aac;
pub mod flac;

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
    pub avc: bool,
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

use bytes::Bytes;

pub mod aac;
pub mod flac;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioType {
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
}

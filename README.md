# Access Unit

[![CI](https://github.com/wavey-ai/access-unit/actions/workflows/ci.yml/badge.svg)](https://github.com/wavey-ai/access-unit/actions/workflows/ci.yml)

A Rust library for handling audio codec frames, with support for AAC and FLAC formats. The library provides utilities for frame detection, parsing, and manipulation, with a focus on broadcast and streaming applications.

## Features

- Automatic audio codec detection (AAC, FLAC, MP3) including MP4 container audio detection
- AAC ADTS frame handling and validation
- FLAC frame parsing and manipulation
- MP3 frame header parsing and detection
- Support for FMP4 container format
- Access Unit abstraction for media streaming

## Audio Codec Support

### AAC Support

- ADTS header parsing and validation
- Frame extraction and manipulation
- Support for common profiles (AAC-LC, HE-AAC v1/v2)
- Sample rate detection and validation
- Channel configuration handling

### FLAC Support

- Frame header parsing
- Block size handling
- Sample rate and channel configuration
- Bits per sample validation
- UTF-8 coded sample/frame numbers
- CRC validation

### MP3 Support

- Frame header validation (sync, version, layer, bitrate, sample rate)
- Frame length calculation
- Frame detection within arbitrary byte streams

## Usage

### Detecting Audio Type

```rust
use audio_codec_handler::detect_audio;

let data: &[u8] = // your audio data
let audio_type = detect_audio(data);

match audio_type {
    AudioType::AAC => println!("AAC audio detected"),
    AudioType::FLAC => println!("FLAC audio detected"),
    AudioType::MP3 => println!("MP3 audio detected"),
    AudioType::Unknown => println!("Unknown audio format"),
    _ => println!("Other format")
}
```

### Detecting MP4 Container Audio

```rust
use audio_codec_handler::mp4;

let data: &[u8] = // MP4 data

if mp4::is_mp4(data) {
    if let Some(audio_type) = mp4::detect_audio_track(data) {
        println!("Audio track codec: {:?}", audio_type);
    }
}
```

### Working with AAC Frames

```rust
use audio_codec_handler::aac;

// Check if data is valid AAC
if aac::is_aac(data) {
    // Extract AAC frame data
    let aac_data = aac::extract_aac_data(&bytes_data);

    // Ensure proper ADTS header
    let processed_data = aac::ensure_adts_header(
        bytes_data,
        channels,
        sample_rate
    );
}
```

### FLAC Frame Handling

```rust
use audio_codec_handler::flac;

// Parse FLAC frame header
let frame_info = flac::decode_frame_header(data)?;

// Split FLAC stream into frames
let frames = flac::split_flac_frames(data);

// Extract single FLAC frame
let frame = flac::extract_flac_frame(data);

// Create STREAMINFO metadata block
let streaminfo = flac::create_streaminfo(&frame_info);
```

### MP3 Frame Detection

```rust
use audio_codec_handler::mp3;

if let Some((offset, header)) = mp3::find_frame(data) {
    println!("Found MP3 frame at offset {} with length {}", offset, header.frame_length);
}
```

### Access Unit Handling

```rust
use audio_codec_handler::AccessUnit;

let unit = AccessUnit {
    key: true,
    pts: 0,
    dts: 0,
    data: data_bytes,
    avc: false,
    id: 1234
};
```

## Frame Header Formats

### AAC ADTS Header

```
|--- Sync Word (12 bits) ---|
|--- ID (1 bit) ---|
|--- Layer (2 bits) ---|
|--- Protection Absent (1 bit) ---|
|--- Profile (2 bits) ---|
|--- Sampling Frequency Index (4 bits) ---|
|--- Private Bit (1 bit) ---|
|--- Channel Configuration (3 bits) ---|
|--- Original/Copy (1 bit) ---|
|--- Home (1 bit) ---|
```

### FLAC Frame Header

```
|--- Sync Code (15 bits) ---|
|--- Reserved (1 bit) ---|
|--- Block Size (4 bits) ---|
|--- Sample Rate (4 bits) ---|
|--- Channel Assignment (4 bits) ---|
|--- Sample Size (3 bits) ---|
|--- Reserved (1 bit) ---|
```

## License

MIT

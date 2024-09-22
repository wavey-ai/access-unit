use std::fmt;

#[derive(Debug, Default)]
pub struct FLACFrameInfo {
    pub is_var_size: bool,
    pub blocking_strategy: u8,
    pub block_size: u16,
    pub sample_rate: u32,
    pub ch_mode: u8,
    pub channels: u8,
    pub bps: u8,
    pub frame_or_sample_num: u64,
}

#[derive(Debug)]
pub enum FLACError {
    InvalidSyncCode,
    InvalidChannelMode(u8),
    InvalidSampleSizeCode(u8),
    InvalidPadding,
    UTF8DecodingError,
    ReservedBlocksizeCode,
    IllegalSampleRateCode(u8),
    UnexpectedEndOfInput,
}

impl fmt::Display for FLACError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FLACError::InvalidSyncCode => write!(f, "Invalid sync code"),
            FLACError::InvalidChannelMode(mode) => write!(f, "Invalid channel mode: {}", mode),
            FLACError::InvalidSampleSizeCode(code) => {
                write!(f, "Invalid sample size code: {}", code)
            }
            FLACError::InvalidPadding => write!(f, "Invalid padding"),
            FLACError::UTF8DecodingError => write!(f, "UTF-8 decoding error"),
            FLACError::ReservedBlocksizeCode => write!(f, "Reserved blocksize code"),
            FLACError::IllegalSampleRateCode(code) => {
                write!(f, "Illegal sample rate code: {}", code)
            }
            FLACError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
        }
    }
}

impl std::error::Error for FLACError {}

const SAMPLE_SIZE_TABLE: [u8; 8] = [0, 8, 12, 0, 16, 20, 24, 32];
const FLAC_BLOCKSIZE_TABLE: [u16; 16] = [
    0, 192, 576, 1152, 2304, 4608, 0, 0, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
];
const FLAC_SAMPLE_RATE_TABLE: [u32; 12] = [
    0, 88200, 176400, 192000, 8000, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

pub fn is_flac(input: &[u8]) -> bool {
    let mut reader = BitReader::new(input);

    match reader.read(15) {
        Ok(b) => b == 0x7FFC,
        Err(_) => false,
    }
}

pub fn decode_frame_header(input: &[u8]) -> Result<FLACFrameInfo, FLACError> {
    let mut reader = BitReader::new(input);
    let mut fi = FLACFrameInfo::default();

    // Frame sync code
    if reader.read(15)? != 0x7FFC {
        return Err(FLACError::InvalidSyncCode);
    }

    // Variable block size stream code
    fi.is_var_size = reader.read_bit()?;
    fi.blocking_strategy = fi.is_var_size as u8;

    // Block size and sample rate codes
    let bs_code = reader.read(4)? as u8;
    let sr_code = reader.read(4)? as u8;

    // Channels and decorrelation
    fi.ch_mode = reader.read(4)? as u8;
    if fi.ch_mode < 8 {
        fi.channels = fi.ch_mode + 1;
        fi.ch_mode = 0; // FLAC_CHMODE_INDEPENDENT
    } else if fi.ch_mode < 11 {
        fi.channels = 2;
        fi.ch_mode -= 7;
    } else {
        return Err(FLACError::InvalidChannelMode(fi.ch_mode));
    }

    // Bits per sample
    let bps_code = reader.read(3)? as u8;
    if bps_code == 3 {
        return Err(FLACError::InvalidSampleSizeCode(bps_code));
    }
    fi.bps = SAMPLE_SIZE_TABLE[bps_code as usize];

    // Reserved bit
    if reader.read_bit()? {
        return Err(FLACError::InvalidPadding);
    }

    // Sample or frame count
    fi.frame_or_sample_num = read_utf8(&mut reader)?;

    // Blocksize
    fi.block_size = match bs_code {
        0 => return Err(FLACError::ReservedBlocksizeCode),
        6 => reader.read(8)? as u16 + 1,
        7 => reader.read(16)? as u16 + 1,
        _ => FLAC_BLOCKSIZE_TABLE[bs_code as usize],
    };

    // Sample rate
    fi.sample_rate = match sr_code {
        0..=11 => FLAC_SAMPLE_RATE_TABLE[sr_code as usize],
        12 => (reader.read(8)? as u32) * 1000,
        13 => reader.read(16)? as u32,
        14 => (reader.read(16)? as u32) * 10,
        _ => return Err(FLACError::IllegalSampleRateCode(sr_code)),
    };

    // Header CRC-8 check
    reader.skip(8)?; // Skip CRC for now

    Ok(fi)
}

fn read_utf8(reader: &mut BitReader) -> Result<u64, FLACError> {
    let mut value = 0u64;
    let mut shift = 0;

    loop {
        let byte = reader.read(8)? as u8;
        if shift == 36 && byte & 0xF8 != 0 {
            return Err(FLACError::UTF8DecodingError);
        }
        value |= ((byte & 0x7F) as u64) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
    }

    Ok(value)
}

struct BitReader<'a> {
    data: &'a [u8],
    bit_position: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit_position: 0,
        }
    }

    fn read(&mut self, num_bits: usize) -> Result<u32, FLACError> {
        let mut result = 0u32;
        for _ in 0..num_bits {
            result = (result << 1) | self.read_bit()? as u32;
        }
        Ok(result)
    }

    fn read_bit(&mut self) -> Result<bool, FLACError> {
        let byte_index = self.bit_position / 8;
        let bit_index = 7 - (self.bit_position % 8);

        if byte_index >= self.data.len() {
            return Err(FLACError::UnexpectedEndOfInput);
        }

        let bit = (self.data[byte_index] >> bit_index) & 1;
        self.bit_position += 1;

        Ok(bit == 1)
    }

    fn skip(&mut self, num_bits: usize) -> Result<(), FLACError> {
        self.bit_position += num_bits;
        if self.bit_position / 8 >= self.data.len() {
            return Err(FLACError::UnexpectedEndOfInput);
        }
        Ok(())
    }
}

pub fn split_flac_frames(data: &[u8]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut start_index = 0;

    // Function to check if a slice starts with a valid FLAC sync code
    fn is_flac_sync(slice: &[u8]) -> bool {
        slice.len() >= 2 && slice[0] == 0xFF && (slice[1] & 0xFC) == 0xF8
    }

    // Iterate through the data to find FLAC frame boundaries
    while start_index < data.len() {
        if is_flac_sync(&data[start_index..]) {
            // Find the start of the next frame
            let mut end_index = start_index + 1;
            while end_index < data.len() {
                if is_flac_sync(&data[end_index..]) {
                    break;
                }
                end_index += 1;
            }

            // Add the frame (including its header) to our list
            frames.push(data[start_index..end_index].to_vec());

            // Move to the start of the next frame
            start_index = end_index;
        } else {
            // If we don't find a sync code, move to the next byte
            start_index += 1;
        }
    }

    frames
}

pub fn extract_flac_frame(data: &[u8]) -> &[u8] {
    // Find the start of the FLAC frame
    // FLAC frames typically start with 0xFF (11111111) followed by 0xF8 to 0xFB
    for i in 0..data.len() - 1 {
        if data[i] == 0xFF && (data[i + 1] & 0xFC) == 0xF8 {
            return &data[i..];
        }
    }
    &[] // Return empty slice if no frame is found
}

pub fn create_streaminfo(frame_info: &FLACFrameInfo) -> Vec<u8> {
    let mut streaminfo = Vec::with_capacity(34);

    // Min and max block size
    streaminfo.extend_from_slice(&frame_info.block_size.to_be_bytes());
    streaminfo.extend_from_slice(&frame_info.block_size.to_be_bytes());

    // Min and max frame size (using placeholders)
    streaminfo.extend_from_slice(&[0, 0, 0]); // min_frame_size (24 bits)
    streaminfo.extend_from_slice(&[0, 0, 0]); // max_frame_size (24 bits)

    // Sample rate, channels, bits per sample, and total samples
    let combined = (frame_info.sample_rate & 0xFFFFF) << 12
        | ((u32::from(frame_info.channels) - 1) & 0x7) << 9
        | ((u32::from(frame_info.bps) - 1) & 0x1F) << 4
        | ((frame_info.frame_or_sample_num >> 32) & 0xF) as u32;
    streaminfo.extend_from_slice(&combined.to_be_bytes());

    streaminfo.extend_from_slice(&(frame_info.frame_or_sample_num as u32).to_be_bytes());

    // MD5 signature (using default value of all zeros)
    streaminfo.extend_from_slice(&[0u8; 16]);

    assert_eq!(streaminfo.len(), 34);
    streaminfo
}

mod tests {
    use super::*;
    use std::char::decode_utf16;
    use std::fs::File;
    use std::io::Read;

    fn read_test_file() -> Vec<u8> {
        let mut file = File::open("testdata/s24le.wav.flac").expect("Failed to open test file");
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .expect("Failed to read test file");
        buffer
    }

    #[test]
    fn test_decode_frame_header() {
        let data = read_test_file();
        let frame_info = decode_frame_header(&data).unwrap();

        assert_eq!(frame_info.is_var_size, false);
        assert_eq!(frame_info.blocking_strategy, 0);
        assert_eq!(frame_info.block_size, 4096);
        assert_eq!(frame_info.sample_rate, 44100);
        assert_eq!(frame_info.ch_mode, 3);
        assert_eq!(frame_info.channels, 2);
        assert_eq!(frame_info.bps, 16);
        assert_eq!(frame_info.frame_or_sample_num, 0);
    }

    #[test]
    fn test_split_flac_frames() {
        let data = read_test_file();
        let frames = split_flac_frames(&data);

        assert!(!frames.is_empty(), "Should have at least one frame");
        assert_eq!(frames.len(), 120);
        // Check that each frame starts with a valid FLAC sync code
        for frame in &frames {
            assert!(frame.len() >= 2, "Frame should be at least 2 bytes long");
            assert_eq!(frame[0], 0xFF, "Frame should start with 0xFF");
            assert_eq!(
                frame[1] & 0xFC,
                0xF8,
                "Second byte should match FLAC sync pattern"
            );
        }
    }

    #[test]
    fn test_extract_flac_frame() {
        let data = read_test_file();
        let frame = extract_flac_frame(&data);

        assert!(!frame.is_empty(), "Should extract a non-empty frame");
        assert_eq!(frame[0], 0xFF, "Frame should start with 0xFF");
        assert_eq!(
            frame[1] & 0xFC,
            0xF8,
            "Second byte should match FLAC sync pattern"
        );
    }
}

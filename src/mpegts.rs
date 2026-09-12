//! MPEG transport stream detection.
//!
//! A transport stream has no magic at the front: it is a run of fixed-size
//! packets, each opening with the sync byte `0x47`. The signature is that
//! byte recurring at the packet stride, so a detector has to see at least two
//! packets to tell a stream from a coincidence.
//!
//! Three strides are in the wild:
//!
//! * `188` — the bare MPEG-TS packet;
//! * `192` — M2TS, a 4-byte arrival timestamp before each packet, so the sync
//!   byte is at offset 4 of every packet;
//! * `204` — DVB, 16 bytes of Reed-Solomon FEC after each packet.

/// The transport-stream sync byte.
const SYNC: u8 = 0x47;

/// True when `data` opens with an MPEG transport stream.
pub fn is_mpeg_ts(data: &[u8]) -> bool {
    synced(data, 188, 0) || synced(data, 192, 4) || synced(data, 204, 0)
}

/// True when the sync byte recurs at `stride`, with the first packet's sync
/// byte at `sync`.
///
/// Two packets are the least that means anything; a third is asked for when
/// the buffer has it, so a pair of stray `0x47`s is not mistaken for a
/// stream.
fn synced(data: &[u8], stride: usize, sync: usize) -> bool {
    let at = |index: usize| data.get(sync + stride * index).copied();
    if at(0) != Some(SYNC) {
        return false;
    }
    if at(1) != Some(SYNC) {
        return false;
    }
    match at(2) {
        Some(byte) => byte == SYNC,
        // Only two packets in hand: two in a row is all this buffer can say.
        None => data.len() >= sync + stride + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packets(stride: usize, sync: usize, count: usize) -> Vec<u8> {
        let mut data = vec![0u8; sync + stride * count];
        for index in 0..count {
            data[sync + stride * index] = SYNC;
        }
        data
    }

    #[test]
    fn detects_188_byte_packets() {
        assert!(is_mpeg_ts(&packets(188, 0, 4)));
    }

    #[test]
    fn detects_m2ts_192_byte_packets() {
        assert!(is_mpeg_ts(&packets(192, 4, 4)));
    }

    #[test]
    fn detects_dvb_204_byte_packets() {
        assert!(is_mpeg_ts(&packets(204, 0, 4)));
    }

    #[test]
    fn rejects_a_lone_sync_byte_and_other_data() {
        assert!(!is_mpeg_ts(&[SYNC]));
        assert!(!is_mpeg_ts(b"RIFF\0\0\0\0WAVE"));
        // Two packets is enough; one sync byte among a stride of noise is not.
        let mut one = vec![0u8; 188 * 2];
        one[0] = SYNC;
        assert!(!is_mpeg_ts(&one));
    }
}

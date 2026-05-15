use crate::packet::TcpPayload;

const MAX_FRAME_LEN: usize = 5_000_000;
const MAX_GAP_FILL: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    C2S,
    S2C,
}

#[derive(Debug)]
pub struct AppFrame {
    pub direction: Direction,
    pub ordinal: u64,
    pub stream_offset: u64,
    pub raw: Vec<u8>,
    pub decoded: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct TcpReassembler {
    c2s: StreamReassembler,
    s2c: StreamReassembler,
}

impl TcpReassembler {
    pub fn feed(&mut self, payload: &TcpPayload<'_>) -> Vec<AppFrame> {
        match payload.direction {
            Direction::C2S => self.c2s.feed(payload),
            Direction::S2C => self.s2c.feed(payload),
        }
    }
}

#[derive(Debug, Default)]
struct StreamReassembler {
    expected_seq: Option<u32>,
    buffer: Vec<u8>,
    stream_offset: u64,
    ordinal: u64,
}

impl StreamReassembler {
    fn feed(&mut self, payload: &TcpPayload<'_>) -> Vec<AppFrame> {
        let mut data = payload.payload;
        match self.expected_seq {
            None => {
                self.expected_seq = Some(payload.seq.wrapping_add(data.len() as u32));
            }
            Some(expected) if payload.seq == expected => {
                self.expected_seq = Some(expected.wrapping_add(data.len() as u32));
            }
            Some(expected) if payload.seq < expected => {
                let overlap = expected.wrapping_sub(payload.seq) as usize;
                if overlap >= data.len() {
                    return Vec::new();
                }
                data = &data[overlap..];
                self.expected_seq = Some(expected.wrapping_add(data.len() as u32));
            }
            Some(expected) => {
                let gap = payload.seq.wrapping_sub(expected) as usize;
                if gap <= MAX_GAP_FILL {
                    self.buffer.resize(self.buffer.len() + gap, 0);
                } else {
                    self.buffer.clear();
                }
                self.expected_seq = Some(payload.seq.wrapping_add(data.len() as u32));
            }
        }

        self.buffer.extend_from_slice(data);
        self.extract_frames(payload.direction)
    }

    fn extract_frames(&mut self, direction: Direction) -> Vec<AppFrame> {
        let mut frames = Vec::new();
        loop {
            if self.buffer.len() < 4 {
                break;
            }

            let length = u32::from_le_bytes([
                self.buffer[0],
                self.buffer[1],
                self.buffer[2],
                self.buffer[3],
            ]) as usize;

            if !(4..=MAX_FRAME_LEN).contains(&length) {
                self.buffer.remove(0);
                self.stream_offset += 1;
                continue;
            }

            if self.buffer.len() < length {
                break;
            }

            let raw = self.buffer.drain(..length).collect::<Vec<_>>();
            let decoded = adjacent_xor(&raw);
            frames.push(AppFrame {
                direction,
                ordinal: self.ordinal,
                stream_offset: self.stream_offset,
                raw,
                decoded,
            });
            self.stream_offset += length as u64;
            self.ordinal += 1;
        }
        frames
    }
}

pub fn frames_from_ordered_stream(direction: Direction, data: &[u8]) -> Vec<AppFrame> {
    let mut frames = Vec::new();
    let mut offset = 0usize;
    let mut ordinal = 0u64;
    while offset + 4 <= data.len() {
        let length = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        if !(4..=MAX_FRAME_LEN).contains(&length) || offset + length > data.len() {
            offset += 1;
            continue;
        }
        let raw = data[offset..offset + length].to_vec();
        let decoded = adjacent_xor(&raw);
        frames.push(AppFrame {
            direction,
            ordinal,
            stream_offset: offset as u64,
            raw,
            decoded,
        });
        offset += length;
        ordinal += 1;
    }
    frames
}

fn adjacent_xor(frame: &[u8]) -> Vec<u8> {
    if frame.len() <= 7 {
        return Vec::new();
    }
    frame[6..]
        .windows(2)
        .map(|pair| pair[0] ^ pair[1])
        .collect()
}

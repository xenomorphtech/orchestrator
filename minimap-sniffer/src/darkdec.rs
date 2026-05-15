use crate::tcp_reassembly::{AppFrame, Direction};

#[derive(Clone, Debug)]
pub enum DecodedUpdate {
    Player(PositionUpdate),
    Entity(EntityUpdate),
}

#[derive(Clone, Debug)]
pub struct PositionUpdate {
    pub t: f64,
    pub x: f32,
    pub z: f32,
    pub rot: f32,
    pub frame_ordinal: u64,
}

#[derive(Clone, Debug)]
pub struct EntityUpdate {
    pub id: u8,
    pub t: f64,
    pub x: f32,
    pub z: f32,
    pub rot: f32,
    pub frame_ordinal: u64,
}

pub fn decode_frame(frame: &AppFrame, now: f64) -> Option<DecodedUpdate> {
    if frame.direction != Direction::S2C || frame.raw.len() != 41 || frame.decoded.len() < 29 {
        return None;
    }

    let decoded = frame.decoded.as_slice();
    let x = f32_at(decoded, 9)?;
    let z = f32_at(decoded, 17)?;
    let rot = f32_at(decoded, 25)?;
    if !ok_coord(x) || !ok_coord(z) || !ok_coord(rot) {
        return None;
    }

    if decoded.starts_with(&[0x12, 0x02, 0x60, 0x6d]) {
        return Some(DecodedUpdate::Player(PositionUpdate {
            t: now,
            x,
            z,
            rot,
            frame_ordinal: frame.ordinal,
        }));
    }

    if decoded[0] == 0x12
        && decoded[2..7] == [0x86, 0x01, 0x00, 0x00, 0x00]
        && decoded[7] == 0x46
        && decoded[8] == 0x11
    {
        return Some(DecodedUpdate::Entity(EntityUpdate {
            id: decoded[1],
            t: now,
            x,
            z,
            rot,
            frame_ordinal: frame.ordinal,
        }));
    }

    None
}

fn f32_at(data: &[u8], offset: usize) -> Option<f32> {
    let bytes: [u8; 4] = data.get(offset..offset + 4)?.try_into().ok()?;
    Some(f32::from_le_bytes(bytes))
}

fn ok_coord(value: f32) -> bool {
    value.is_finite() && value.abs() < 100_000.0
}

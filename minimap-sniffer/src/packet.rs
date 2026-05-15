use crate::tcp_reassembly::Direction;

#[derive(Clone, Copy, Debug)]
pub struct TcpPayload<'a> {
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub direction: Direction,
    pub payload: &'a [u8],
}

pub fn parse_ethernet_ipv4_tcp<'a>(frame: &'a [u8], game_port: u16) -> Option<TcpPayload<'a>> {
    if frame.len() < 14 {
        return None;
    }

    let mut cursor = 12usize;
    let mut eth_type = be_u16(frame, cursor)?;
    cursor += 2;
    while eth_type == 0x8100 || eth_type == 0x88a8 {
        if frame.len() < cursor + 4 {
            return None;
        }
        cursor += 2;
        eth_type = be_u16(frame, cursor)?;
        cursor += 2;
    }

    if eth_type != 0x0800 {
        return None;
    }

    let ip = cursor;
    if frame.len() < ip + 20 || frame[ip] >> 4 != 4 {
        return None;
    }
    let ihl = usize::from(frame[ip] & 0x0f) * 4;
    if ihl < 20 || frame.len() < ip + ihl || frame[ip + 9] != 6 {
        return None;
    }

    let total_len = usize::from(be_u16(frame, ip + 2)?);
    if total_len < ihl || frame.len() < ip + total_len {
        return None;
    }

    let frag = be_u16(frame, ip + 6)?;
    if frag & 0x1fff != 0 {
        return None;
    }

    let tcp = ip + ihl;
    if frame.len() < tcp + 20 {
        return None;
    }

    let src_port = be_u16(frame, tcp)?;
    let dst_port = be_u16(frame, tcp + 2)?;
    if src_port != game_port && dst_port != game_port {
        return None;
    }

    let data_offset = usize::from(frame[tcp + 12] >> 4) * 4;
    if data_offset < 20 || tcp + data_offset > ip + total_len {
        return None;
    }

    let payload = &frame[tcp + data_offset..ip + total_len];
    if payload.is_empty() {
        return None;
    }

    let direction = if src_port == game_port {
        Direction::S2C
    } else {
        Direction::C2S
    };

    let mut src = [0u8; 4];
    let mut dst = [0u8; 4];
    src.copy_from_slice(&frame[ip + 12..ip + 16]);
    dst.copy_from_slice(&frame[ip + 16..ip + 20]);

    Some(TcpPayload {
        src,
        dst,
        src_port,
        dst_port,
        seq: be_u32(frame, tcp + 4)?,
        direction,
        payload,
    })
}

fn be_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(data.get(offset..offset + 2)?.try_into().ok()?))
}

fn be_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(data.get(offset..offset + 4)?.try_into().ok()?))
}

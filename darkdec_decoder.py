from __future__ import annotations

import argparse
import csv
import json
import math
import socket
import struct
from collections import Counter, defaultdict
from pathlib import Path

DEFAULT_PCAP = 'CAPTURA DARK DECEMBER PRIMERA QUEST.pcapng'
DEFAULT_PORT = 10001


def read_pcapng(path: Path):
    data = path.read_bytes()
    endian = '<'
    off = 0
    packets = []
    interfaces = []
    resolutions = []
    blocks = Counter()
    while off + 12 <= len(data):
        block_type, block_len = struct.unpack_from(endian + 'II', data, off)
        if block_len < 12 or off + block_len > len(data):
            break
        blocks[block_type] += 1
        body = data[off + 8:off + block_len - 4]
        if block_type == 0x0A0D0D0A:
            endian = '<' if struct.unpack_from('<I', data, off + 8)[0] == 0x1A2B3C4D else '>'
        elif block_type == 1 and len(body) >= 8:
            link_type, _reserved, snap_len = struct.unpack_from(endian + 'HHI', body, 0)
            interfaces.append((link_type, snap_len))
            resolutions.append(1e-6)
            pos = 8
            while pos + 4 <= len(body):
                code, length = struct.unpack_from(endian + 'HH', body, pos)
                pos += 4
                value = body[pos:pos + length]
                pos += (length + 3) & ~3
                if code == 0:
                    break
                if code == 9 and value:
                    byte = value[0]
                    resolutions[-1] = 2 ** -(byte & 0x7f) if byte & 0x80 else 10 ** -byte
        elif block_type == 6 and len(body) >= 20:
            iface, high, low, cap_len, _orig_len = struct.unpack_from(endian + 'IIIII', body, 0)
            scale = resolutions[iface] if iface < len(resolutions) else 1e-6
            packets.append((((high << 32) | low) * scale, body[20:20 + cap_len]))
        off += block_len
    return packets, interfaces, blocks


def tcp_payloads(packets, port):
    rows = []
    for index, (ts, pkt) in enumerate(packets):
        if len(pkt) < 54:
            continue
        eth_type = struct.unpack_from('!H', pkt, 12)[0]
        ip = 14
        if eth_type == 0x8100:
            eth_type = struct.unpack_from('!H', pkt, 16)[0]
            ip = 18
        if eth_type != 0x0800 or pkt[ip] >> 4 != 4:
            continue
        ihl = (pkt[ip] & 15) * 4
        if pkt[ip + 9] != 6:
            continue
        ip_len = struct.unpack_from('!H', pkt, ip + 2)[0]
        src = socket.inet_ntoa(pkt[ip + 12:ip + 16])
        dst = socket.inet_ntoa(pkt[ip + 16:ip + 20])
        tcp = ip + ihl
        sport, dport = struct.unpack_from('!HH', pkt, tcp)
        if sport != port and dport != port:
            continue
        data_off = (pkt[tcp + 12] >> 4) * 4
        seq = struct.unpack_from('!I', pkt, tcp + 4)[0]
        payload = pkt[tcp + data_off:ip + ip_len]
        if payload:
            direction = 'S2C' if sport == port else 'C2S'
            rows.append((direction, seq, index, ts, src, sport, dst, dport, payload))
    return rows


def adjacent_xor(frame):
    body = frame[6:]
    return bytes(body[i] ^ body[i + 1] for i in range(len(body) - 1))


def app_frames(payloads):
    result = []
    first_ts = min((row[3] for row in payloads), default=0.0)
    for direction in ('C2S', 'S2C'):
        rows = sorted([row for row in payloads if row[0] == direction], key=lambda row: (row[1], row[2]))
        buf = bytearray()
        expect = None
        stream_off = 0
        ordinal = 0
        for row in rows:
            seq = row[1]
            ts = row[3]
            payload = row[8]
            if expect is None:
                expect = seq
            if seq < expect:
                payload = payload[expect - seq:]
            elif seq > expect:
                buf.extend(bytes(seq - expect))
            expect = seq + len(row[8])
            buf.extend(payload)
            while len(buf) >= 4:
                length = struct.unpack_from('<I', buf, 0)[0]
                if length < 4 or length > 5000000:
                    raise ValueError(f'bad frame length {length} in {direction}')
                if len(buf) < length:
                    break
                raw = bytes(buf[:length])
                result.append({'dir': direction, 'ord': ordinal, 'off': stream_off, 't': ts - first_ts, 'raw': raw, 'dec': adjacent_xor(raw)})
                del buf[:length]
                stream_off += length
                ordinal += 1
    return sorted(result, key=lambda row: (row['t'], row['dir'], row['ord']))


def f32(data, offset):
    return struct.unpack_from('<f', data, offset)[0]


def ok_coord(*values):
    return all(math.isfinite(value) and abs(value) < 100000 for value in values)


def extract_tracks(frames):
    player = []
    entities = defaultdict(list)
    shape = bytes.fromhex('86 01 00 00 00')
    for frame in frames:
        dec = frame['dec']
        if frame['dir'] != 'S2C' or len(frame['raw']) != 41 or len(dec) < 29:
            continue
        if dec.startswith(bytes.fromhex('12 02 60 6d')):
            x = f32(dec, 9)
            z = f32(dec, 17)
            rot = f32(dec, 25)
            if ok_coord(x, z, rot):
                player.append({'t': frame['t'], 'i': frame['ord'], 'x': x, 'z': z, 'rot': rot})
            continue
        if dec[0] == 0x12 and dec[2:7] == shape and dec[7] == 0x46 and dec[8] == 0x11:
            eid = dec[1]
            x = f32(dec, 9)
            z = f32(dec, 17)
            rot = f32(dec, 25)
            if ok_coord(x, z, rot):
                entities[eid].append({'t': frame['t'], 'i': frame['ord'], 'x': x, 'z': z, 'rot': rot})
    return player, entities


def distance(points):
    total = 0.0
    for a, b in zip(points, points[1:]):
        step = math.hypot(b['x'] - a['x'], b['z'] - a['z'])
        if step < 10000:
            total += step
    return total


def summary(points):
    xs = [p['x'] for p in points]
    zs = [p['z'] for p in points]
    return {'count': len(points), 'first_t': points[0]['t'], 'last_t': points[-1]['t'], 'first_i': points[0]['i'], 'last_i': points[-1]['i'], 'start_x': xs[0], 'start_z': zs[0], 'last_x': xs[-1], 'last_z': zs[-1], 'min_x': min(xs), 'max_x': max(xs), 'min_z': min(zs), 'max_z': max(zs), 'distance': distance(points)}


def active_end(player, entities, window=5.0):
    if not player:
        return []
    end = max([player[-1]['t']] + [points[-1]['t'] for points in entities.values() if points])
    you = player[-1]
    rows = []
    for eid, points in entities.items():
        p = points[-1]
        if end - p['t'] > window:
            continue
        dx = p['x'] - you['x']
        dz = p['z'] - you['z']
        rows.append({'entity_id': f'0x{eid:02x}', 'last_t': p['t'], 'age_at_end': end - p['t'], 'x': p['x'], 'z': p['z'], 'dx_from_player': dx, 'dz_from_player': dz, 'distance_from_player': math.hypot(dx, dz)})
    return sorted(rows, key=lambda row: row['distance_from_player'])


def decimate(points, limit):
    if len(points) <= limit:
        return points
    step = max(1, math.ceil(len(points) / limit))
    kept = points[::step]
    if kept[-1] is not points[-1]:
        kept.append(points[-1])
    return kept


def write_csv(path, rows, fields):
    with path.open('w', newline='', encoding='utf-8') as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def write_outputs(out, pcap, packets, interfaces, blocks, payloads, frames, player, entities, port):
    out.mkdir(parents=True, exist_ok=True)
    write_csv(out / 'player_track.csv', player, ['t', 'i', 'x', 'z', 'rot'])
    entity_rows = []
    for eid, points in sorted(entities.items()):
        for point in points:
            row = dict(point)
            row['entity_id'] = f'0x{eid:02x}'
            entity_rows.append(row)
    write_csv(out / 'entity_tracks.csv', entity_rows, ['entity_id', 't', 'i', 'x', 'z', 'rot'])
    sum_rows = []
    for eid, points in sorted(entities.items(), key=lambda item: len(item[1]), reverse=True):
        row = summary(points)
        row['entity_id'] = f'0x{eid:02x}'
        sum_rows.append(row)
    fields = ['entity_id', 'count', 'first_t', 'last_t', 'first_i', 'last_i', 'start_x', 'start_z', 'last_x', 'last_z', 'min_x', 'max_x', 'min_z', 'max_z', 'distance']
    write_csv(out / 'entities_summary.csv', sum_rows, fields)
    write_csv(out / 'active_end_entities.csv', active_end(player, entities), ['entity_id', 'last_t', 'age_at_end', 'x', 'z', 'dx_from_player', 'dz_from_player', 'distance_from_player'])
    with (out / 'frame_lengths.csv').open('w', newline='', encoding='utf-8') as handle:
        writer = csv.writer(handle)
        writer.writerow(['direction', 'frame_length', 'count'])
        for (direction, length), count in sorted(Counter((f['dir'], len(f['raw'])) for f in frames).items()):
            writer.writerow([direction, length, count])
    write_report(out, pcap, packets, interfaces, blocks, payloads, frames, player, entities, port)
    write_minimap(out, pcap.name, player, entities)


def write_report(out, pcap, packets, interfaces, blocks, payloads, frames, player, entities, port):
    counts = Counter((f['dir'], len(f['raw'])) for f in frames)
    p = summary(player) if player else None
    nl = chr(10)
    lines = ['# Dark December capture decode report', '', f'- Capture: `{pcap.name}`', f'- Packets in pcapng: {len(packets):,}', f'- pcapng blocks: {dict(blocks)}', f'- Interfaces: {interfaces}', f'- TCP payload segments on port {port}: {len(payloads):,}', f'- App frames decoded: {len(frames):,}', '', '## Protocol notes', '', '- Frames use a 4-byte little-endian total length.', '- Useful body decode: `decoded[i] = raw_body[i] ^ raw_body[i + 1]`.', '- Player candidate prefix: `12 02 60 6d`.', '- Entity candidate shape: `12 <id> 86 01 00 00 00 46 11`.', '- Coordinates are float32 at decoded offsets 9 and 17.', '', '## Top frame lengths', '', '| Direction | Length | Count |', '|---|---:|---:|']
    for (direction, length), count in sorted(counts.items(), key=lambda item: item[1], reverse=True)[:20]:
        lines.append(f'| {direction} | {length} | {count:,} |')
    lines += ['', '## Player', '']
    if p:
        lines.append('- Updates: {count:,}; time {first_t:.3f}s to {last_t:.3f}s.'.format(**p))
        lines.append('- Start: ({start_x:.2f}, {start_z:.2f}); last: ({last_x:.2f}, {last_z:.2f}).'.format(**p))
        lines.append('- Bounds X {min_x:.2f}..{max_x:.2f}, Z {min_z:.2f}..{max_z:.2f}; decoded path length {distance:.2f}.'.format(**p))
    lines += ['', '## Entity candidates', '', f'- Entity IDs found: {len(entities)}', f'- Entity movement updates: {sum(len(points) for points in entities.values()):,}', '', '| ID | Updates | Last t | Last X | Last Z | Distance |', '|---|---:|---:|---:|---:|---:|']
    for row in sorted([dict(summary(points), entity_id=f'0x{eid:02x}') for eid, points in entities.items()], key=lambda row: row['count'], reverse=True)[:20]:
        lines.append('| {entity_id} | {count:,} | {last_t:.2f} | {last_x:.2f} | {last_z:.2f} | {distance:.2f} |'.format(**row))
    lines += ['', '## Active near capture end', '', '| ID | Last t | X | Z | Distance from player |', '|---|---:|---:|---:|---:|']
    for row in active_end(player, entities)[:20]:
        lines.append('| {entity_id} | {last_t:.2f} | {x:.2f} | {z:.2f} | {distance_from_player:.2f} |'.format(**row))
    (out / 'decode_report.md').write_text(nl.join(lines) + nl, encoding='utf-8')


def write_minimap(out, name, player, entities):
    p = decimate(player, 1800)
    ents = {f'0x{eid:02x}': decimate(points, 700) for eid, points in sorted(entities.items()) if len(points) >= 20}
    all_points = p + [point for points in ents.values() for point in points]
    if all_points:
        minx = min(point['x'] for point in all_points)
        maxx = max(point['x'] for point in all_points)
        minz = min(point['z'] for point in all_points)
        maxz = max(point['z'] for point in all_points)
        padx = max(50, (maxx - minx) * 0.08)
        padz = max(50, (maxz - minz) * 0.08)
    else:
        minx = minz = 0
        maxx = maxz = 1
        padx = padz = 0
    data = {'pcap': name, 'player': p, 'entities': ents, 'bounds': {'minX': minx - padx, 'maxX': maxx + padx, 'minZ': minz - padz, 'maxZ': maxz + padz}, 'duration': max([0] + [x['t'] for x in p] + [x['t'] for pts in ents.values() for x in pts]), 'entityCount': len(entities), 'entityUpdateCount': sum(len(x) for x in entities.values()), 'playerUpdateCount': len(player)}
    template = '''<!doctype html><html><head><meta charset=utf-8><meta name=viewport content=width=device-width,initial-scale=1><title>Dark December minimap</title><style>body{margin:0;background:#10151b;color:#e8edf2;font-family:Segoe UI,Arial,sans-serif}header,footer{padding:10px 14px;background:#1b232c}main{height:calc(100vh - 98px)}canvas{display:block;width:100%;height:100%;background:#0c1117}.row{display:flex;gap:12px;align-items:center;flex-wrap:wrap}input[type=range]{flex:1;min-width:220px}button{background:#26313d;color:#e8edf2;border:1px solid #425061;border-radius:6px;padding:6px 10px}.muted{color:#aab5c0;font-size:13px}</style></head><body><header><b>Dark December decoded minimap</b><span class=muted id=stats></span></header><main><canvas id=map></canvas></main><footer class=row><input id=time type=range min=0 max=1000 value=1000><button id=play>Play</button><label class=muted><input id=trails type=checkbox checked> Trails</label><span class=muted id=readout></span></footer><script>const DATA=__DATA__;const c=document.getElementById('map'),ctx=c.getContext('2d'),s=document.getElementById('time'),r=document.getElementById('readout'),tr=document.getElementById('trails'),btn=document.getElementById('play');document.getElementById('stats').textContent=` ${DATA.playerUpdateCount} player updates, ${DATA.entityCount} entity ids`;let playing=false,last=0;function resize(){const d=devicePixelRatio||1,b=c.getBoundingClientRect();c.width=b.width*d;c.height=b.height*d;ctx.setTransform(d,0,0,d,0,0);draw()}function t(){return DATA.duration*Number(s.value)/1000}function xy(p){const w=c.clientWidth,h=c.clientHeight,b=DATA.bounds;return[(p.x-b.minX)/(b.maxX-b.minX)*w,h-(p.z-b.minZ)/(b.maxZ-b.minZ)*h]}function before(a,t){let z=null;for(const p of a){if(p.t<=t)z=p;else break}return z}function trail(a,t,col,w){ctx.strokeStyle=col;ctx.lineWidth=w;let on=false;ctx.beginPath();for(const p of a){if(p.t>t)break;const q=xy(p);if(!on){ctx.moveTo(q[0],q[1]);on=true}else ctx.lineTo(q[0],q[1])}if(on)ctx.stroke()}function draw(){const now=t(),w=c.clientWidth,h=c.clientHeight;ctx.clearRect(0,0,w,h);ctx.strokeStyle='rgba(255,255,255,.08)';for(let i=0;i<=10;i++){ctx.beginPath();ctx.moveTo(i*w/10,0);ctx.lineTo(i*w/10,h);ctx.stroke();ctx.beginPath();ctx.moveTo(0,i*h/10);ctx.lineTo(w,i*h/10);ctx.stroke()}if(tr.checked){for(const a of Object.values(DATA.entities))trail(a,now,'rgba(255,105,95,.16)',1);trail(DATA.player,now,'rgba(82,169,255,.75)',2.5)}let active=0;for(const [id,a] of Object.entries(DATA.entities)){const p=before(a,now);if(!p||now-p.t>2.5)continue;active++;const q=xy(p);ctx.fillStyle='#ff6b5f';ctx.beginPath();ctx.arc(q[0],q[1],4,0,7);ctx.fill();ctx.fillStyle='#fff';ctx.font='11px Segoe UI';ctx.fillText(id,q[0]+6,q[1]-6)}const you=before(DATA.player,now);if(you){const q=xy(you);ctx.fillStyle='#52a9ff';ctx.beginPath();ctx.moveTo(q[0],q[1]-9);ctx.lineTo(q[0]+8,q[1]+7);ctx.lineTo(q[0]-8,q[1]+7);ctx.closePath();ctx.fill()}r.textContent=`${now.toFixed(1)}s / ${DATA.duration.toFixed(1)}s | active ${active}`}function tick(x){if(!playing)return;if(!last)last=x;const n=Math.min(1000,Number(s.value)+(x-last)/1000*80);last=x;s.value=n;if(n>=1000){playing=false;btn.textContent='Play'}draw();requestAnimationFrame(tick)}s.oninput=draw;tr.onchange=draw;btn.onclick=()=>{playing=!playing;btn.textContent=playing?'Pause':'Play';last=0;if(playing)requestAnimationFrame(tick)};onresize=resize;resize();</script></body></html>'''
    (out / 'minimap.html').write_text(template.replace('__DATA__', json.dumps(data, separators=(',', ':'))), encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description='Decode Dark December port 10001 pcapng traffic')
    parser.add_argument('pcap', nargs='?', default=DEFAULT_PCAP)
    parser.add_argument('--port', type=int, default=DEFAULT_PORT)
    parser.add_argument('--out', default='darkdec_output')
    args = parser.parse_args()
    pcap = Path(args.pcap)
    out = Path(args.out)
    packets, interfaces, blocks = read_pcapng(pcap)
    payloads = tcp_payloads(packets, args.port)
    frames = app_frames(payloads)
    player, entities = extract_tracks(frames)
    write_outputs(out, pcap, packets, interfaces, blocks, payloads, frames, player, entities, args.port)
    print(f'packets={len(packets)} segments={len(payloads)} frames={len(frames)}')
    print(f'player_updates={len(player)} entity_ids={len(entities)} entity_updates={sum(len(v) for v in entities.values())}')
    print(f'wrote={out.resolve()}')


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Minimal pcap (LINUX_SLL2) parser: reassemble TCP streams on port 8300 and
decode VESC-style frames (0x02 len payload crc16 0x03 | 0x03 len16 ...)."""
import struct, sys, collections

PORT = 8300
data = open(sys.argv[1], 'rb').read()
magic = struct.unpack('<I', data[:4])[0]
assert magic in (0xa1b2c3d4, 0xa1b23c4d), hex(magic)
linktype = struct.unpack('<I', data[20:24])[0]
assert linktype == 276, linktype  # LINUX_SLL2
tsdiv = 1e9 if magic == 0xa1b23c4d else 1e6

def crc16(buf):
    crc = 0
    for b in buf:
        crc ^= b << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) & 0xFFFF if crc & 0x8000 else (crc << 1) & 0xFFFF
    return crc

# streams[(src, sport, dst, dport)] = list of (ts, seq, payload)
streams = collections.defaultdict(list)
udp = collections.Counter()
off = 24
t0 = None
while off + 16 <= len(data):
    ts_s, ts_f, caplen, _ = struct.unpack('<IIII', data[off:off + 16])
    pkt = data[off + 16:off + 16 + caplen]
    off += 16 + caplen
    ts = ts_s + ts_f / tsdiv
    if t0 is None:
        t0 = ts
    proto, _, ifidx, _, pkttype, _ = struct.unpack('>HHIHBB', pkt[:12])
    if proto != 0x0800:
        continue
    ip = pkt[20:]
    ihl = (ip[0] & 0xF) * 4
    ipproto = ip[9]
    src = '.'.join(map(str, ip[12:16])); dst = '.'.join(map(str, ip[16:20]))
    l4 = ip[ihl:]
    if ipproto == 6:
        sport, dport, seq = struct.unpack('>HHI', l4[:8])
        thl = (l4[12] >> 4) * 4
        pl = l4[thl:ip_total] if False else l4[thl:]
        totlen = struct.unpack('>H', ip[2:4])[0]
        pl = pl[:totlen - ihl - thl]
        if PORT in (sport, dport) and pl:
            streams[(src, sport, dst, dport)].append((ts - t0, seq, pl))
    elif ipproto == 17:
        sport, dport = struct.unpack('>HH', l4[:4])
        if PORT in (sport, dport):
            udp[(src, sport, dst, dport)] += 1

print("UDP-flöden på 8300:", dict(udp) or "inga")
print()

def reassemble(chunks):
    seen = {}
    for ts, seq, pl in chunks:
        seen.setdefault(seq, (ts, pl))
    out = bytearray(); marks = []  # (byte_offset, ts)
    exp = None
    for seq in sorted(seen):
        ts, pl = seen[seq]
        if exp is not None and seq < exp:
            pl = pl[exp - seq:]
            seq = exp
        if not pl:
            continue
        marks.append((len(out), ts))
        out += pl; exp = seq + len(pl)
    return bytes(out), marks

def frames(buf):
    i = 0; res = []; bad = 0
    while i < len(buf):
        b = buf[i]
        if b == 0x02 and i + 1 < len(buf):
            n = buf[i + 1]; hdr = 2
        elif b == 0x03 and i + 2 < len(buf):
            n = (buf[i + 1] << 8) | buf[i + 2]; hdr = 3
        else:
            bad += 1; i += 1; continue
        end = i + hdr + n + 3
        if end > len(buf):
            break
        payload = buf[i + hdr:i + hdr + n]
        crc = (buf[i + hdr + n] << 8) | buf[i + hdr + n + 1]
        stop = buf[i + hdr + n + 2]
        if stop == 0x03 and crc == crc16(payload):
            res.append((i, payload)); i = end
        else:
            bad += 1; i += 1
    return res, bad

for key, chunks in sorted(streams.items(), key=lambda kv: -sum(len(c[2]) for c in kv[1])):
    buf, marks = reassemble(chunks)
    fr, bad = frames(buf)
    print(f"=== {key[0]}:{key[1]} -> {key[2]}:{key[3]}  bytes={len(buf)} giltiga ramar={len(fr)} resync-byte={bad}")
    ids = collections.Counter(p[0] for _, p in fr if p)
    for cid, c in sorted(ids.items()):
        first = next(p for _, p in fr if p and p[0] == cid)
        lens = sorted({len(p) for _, p in fr if p and p[0] == cid})
        print(f"   cmd 0x{cid:02x} ({cid:3d})  x{c:5d}  payloadlängder={lens[:6]}  ex={first[:24].hex(' ')}")
    print()

# --- andra nivån: [car_id][cmd][args...] ---
print("############ per-kommando (byte1) ############")
for key, chunks in sorted(streams.items(), key=lambda kv: -sum(len(c[2]) for c in kv[1])):
    buf, marks = reassemble(chunks)
    fr, _ = frames(buf)
    print(f"=== {key[0]}:{key[1]} -> {key[2]}:{key[3]}")
    tally = collections.defaultdict(list)
    for _, p in fr:
        if len(p) >= 2:
            tally[p[1]].append(p)
    for cmd, ps in sorted(tally.items()):
        lens = collections.Counter(len(p) for p in ps)
        uniq = []
        for p in ps:
            if p not in uniq: uniq.append(p)
            if len(uniq) >= 4: break
        print(f"   cmd 0x{cmd:02x} ({cmd:3d}) x{len(ps):5d} längder={dict(lens)}")
        for u in uniq[:4]:
            print("        ", u[:40].hex(' '))

print("############ tidslinje 0x7d ############")
import bisect
for key, chunks in streams.items():
    if key[3] != PORT: continue
    buf, marks = reassemble(chunks)
    fr, _ = frames(buf)
    offs = [m[0] for m in marks]
    for off_, p in fr:
        if len(p) >= 2 and p[1] == 0x7d:
            ts = marks[bisect.bisect_right(offs, off_) - 1][1]
            val = int.from_bytes(p[3:7], 'big', signed=True)
            print(f"  t={ts:7.2f}s  {p.hex(' ')}   arg0={p[2]}  int32={val}")

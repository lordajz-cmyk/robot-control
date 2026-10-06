#!/usr/bin/env python3
"""Låtsas-Car_Client på port 8300 för att prova RControlStations statusruta utan robot.

  python3 scripts/lattsasbil_car_client.py 127.0.0.2 60      (adress, sekunder)
  RControlStation --connecttcp 127.0.0.2:8300 --addcar 0:1

Svarar på CMD_GET_STATE (fw 30.3, 52,3 V, roll 18°, felkod 2), CMD_GET_VESC_STATUS (140)
och terminalkommandot "vinkel"."""
import socket, struct, sys, threading, time

def crc16(b):
    c = 0
    for x in b:
        c ^= x << 8
        for _ in range(8):
            c = ((c << 1) ^ 0x1021) if c & 0x8000 else (c << 1)
            c &= 0xFFFF
    return c

def frame(p):
    h = bytes([2, len(p)]) if len(p) <= 255 else bytes([3]) + struct.pack(">H", len(p))
    return h + p + struct.pack(">H", crc16(p)) + b"\x03"

def f32(v, s): return struct.pack(">i", int(v * s))

def state(cid):
    p = bytes([cid, 120, 30, 3])
    p += f32(18.0, 1e6) + f32(-3.0, 1e6) + f32(123.0, 1e6)
    p += f32(0, 1e6) * 9
    p += f32(0, 1e4) * 2
    p += f32(0.9, 1e6) + f32(16 * 3.27, 1e6) + f32(34.0, 1e6) + bytes([2])
    p += f32(0, 1e4) * 4 + f32(0, 1e6) + struct.pack(">i", 0) + struct.pack(">h", 0)
    p += f32(0, 1e4) * 4 + struct.pack(">H", 0)
    return p

def vesc(cid):
    p = bytes([cid, 140])
    for i, cur in ((90, 40), (87, 38)):
        p += bytes([i]) + struct.pack(">HihH", 50, 1500, cur, 400)
    return p

log = []; RAW = []
def handle(c):
    buf = b""
    while True:
        d = c.recv(4096)
        if not d:
            return
        buf += d; RAW.append(d)
        while len(buf) >= 2:
            if buf[0] == 2:
                n, h = buf[1], 2
            elif buf[0] == 3 and len(buf) >= 3:
                n, h = struct.unpack(">H", buf[1:3])[0], 3
            else:
                buf = buf[1:]; continue
            if len(buf) < h + n + 3:
                break
            p = buf[h:h + n]; buf = buf[h + n + 3:]
            cid, cmd = p[0], p[1]
            log.append(cmd)
            if cmd == 120:
                c.sendall(frame(state(cid)))
            elif cmd == 140:
                c.sendall(frame(vesc(cid)))
            elif cmd == 1:
                txt = p[2:].decode("latin1")
                if txt == "vinkel":
                    c.sendall(frame(bytes([cid, 0]) + b"Vinkelgivare: 2100 mV, vinkel -10.0 grader, OK"))

s = socket.socket(); s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind((sys.argv[1], 8300)); s.listen(1)
end = time.time() + float(sys.argv[2])
s.settimeout(1)
while time.time() < end:
    try:
        c, _ = s.accept()
    except socket.timeout:
        continue
    threading.Thread(target=handle, args=(c,), daemon=True).start()
from collections import Counter
print("kommandon:", Counter(log)); print("rått:", sum(map(len,RAW)), b"".join(RAW)[:60].hex())

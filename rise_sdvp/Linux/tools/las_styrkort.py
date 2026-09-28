#!/usr/bin/env python3
"""las_styrkort.py — läser ett styrkorts inställningar (samma som Read i
RControlStation:s Confcommon) och skriver ut dem som en textrad att mejla.

Skickar BARA läsfrågan CMD_GET_MAIN_CONFIG (78) till ID 255 ("alla"), via
Car_Client. Ändrar ingenting på kortet. Bara Pythons standardbibliotek.

  python3 las_styrkort.py 192.168.200.7        via TCP (RControlStation ska vara frånkopplad,
                                               Car_Client tar bara en TCP-klient)
  python3 las_styrkort.py 127.0.0.1            på själva Jetson/Pi:n
  python3 las_styrkort.py 192.168.200.7 --udp  via UDP (kräver Car_Client --useudp),
                                               RControlStation kan vara ansluten

Resultatet skrivs ut och sparas i styrkort_<tid>.txt i aktuell mapp.
"""
import socket
import sys
import time

PORT = 8300
CMD_GET_MAIN_CONFIG = 78
FRAGA = bytes([255, CMD_GET_MAIN_CONFIG])  # det enda som någonsin skickas
TIDSGRANS = 6.0


def crc16(data):
    crc = 0
    for b in data:
        crc ^= b << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) if crc & 0x8000 else (crc << 1)
            crc &= 0xFFFF
    return crc


def rama_in(payload):
    if len(payload) <= 255:
        head = bytes([2, len(payload)])
    else:
        head = bytes([3, len(payload) >> 8, len(payload) & 0xFF])
    c = crc16(payload)
    return head + payload + bytes([c >> 8, c & 0xFF, 3])


def plocka_paket(buf):
    """Returnerar (lista med payloads, rest av bufferten)."""
    ut = []
    while buf:
        if buf[0] == 2 and len(buf) >= 2:
            n, h = buf[1], 2
        elif buf[0] == 3 and len(buf) >= 3:
            n, h = (buf[1] << 8) | buf[2], 3
        elif buf[0] in (2, 3):
            break  # ofullständigt huvud
        else:
            buf = buf[1:]
            continue
        total = h + n + 3
        if len(buf) < total:
            break
        p = buf[h:h + n]
        if buf[total - 1] == 3 and crc16(p) == ((buf[h + n] << 8) | buf[h + n + 1]):
            ut.append(p)
            buf = buf[total:]
        else:
            buf = buf[1:]
    return ut, buf


def via_tcp(host):
    s = socket.create_connection((host, PORT), timeout=TIDSGRANS)
    s.sendall(rama_in(FRAGA))
    buf = b""
    slut = time.time() + TIDSGRANS
    try:
        while time.time() < slut:
            s.settimeout(max(0.1, slut - time.time()))
            try:
                d = s.recv(4096)
            except socket.timeout:
                break
            if not d:
                raise SystemExit("Car_Client stängde anslutningen direkt. Är RControlStation ansluten? "
                                 "Koppla från den, eller kör med --udp.")
            buf += d
            paket, buf = plocka_paket(buf)
            for p in paket:
                if len(p) >= 2 and p[1] == CMD_GET_MAIN_CONFIG:
                    return p
    finally:
        s.close()
    return None


def via_udp(host):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.bind(("0.0.0.0", PORT + 1))  # Car_Client svarar till avsändaren, port 8301
    s.sendto(FRAGA, (host, PORT))
    slut = time.time() + TIDSGRANS
    try:
        while time.time() < slut:
            s.settimeout(max(0.1, slut - time.time()))
            try:
                p, _ = s.recvfrom(4096)
            except socket.timeout:
                break
            if len(p) >= 2 and p[1] == CMD_GET_MAIN_CONFIG:
                return p
    finally:
        s.close()
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    host = args[0] if args else "127.0.0.1"
    udp = "--udp" in sys.argv
    print("Läser styrkortets inställningar via Car_Client på %s (%s)..." % (host, "UDP" if udp else "TCP"))
    try:
        svar = via_udp(host) if udp else via_tcp(host)
    except OSError as e:
        raise SystemExit("Kunde inte nå Car_Client på %s:%d: %s" % (host, PORT, e))
    if not svar:
        raise SystemExit("Inget svar från styrkortet inom %.0f s." % TIDSGRANS)

    tid = time.strftime("%Y-%m-%d_%H-%M-%S")
    rad = "STYRKORT %s %s id=%d len=%d %s" % (tid, host, svar[0], len(svar), svar.hex())
    print()
    print(rad)
    print()
    fil = "styrkort_%s.txt" % tid
    with open(fil, "w") as f:
        f.write(rad + "\n")
    print("Sparat i %s. Mejla hela raden som börjar med STYRKORT." % fil)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""testa_write.py — testar att styrkortet klarar Write (Confcommon i RControlStation).

Gör exakt det RControlStations Write gör, men skriver tillbaka kortets EGNA
värden (inget ändras): läser inställningarna, skickar samma bytes som
RControlStation skickar (t.o.m. aktuatorerna) och läser igen för att se att
kortet lever. Upprepas flera gånger.

Bakgrund (2026-09-28): kortet läste förbi slutet på RControlStations paket och
sparade GPS-text som "reglerloopar". Nästa Write slog på dem och kortet hängde
(HardFault). Med gammal firmware hänger kortet därför på andra skrivningen.

Körs på Pi:n (RControlStation ska vara frånkopplad, Car_Client tar en TCP-klient):
  python3 testa_write.py            3 skrivningar mot 127.0.0.1
  python3 testa_write.py 5          5 skrivningar
Hänger kortet: starta om det över ST-Link (se MACTRAC_STATUS.md).
"""
import socket
import struct
import sys
import time

PORT = 8300
CMD_SET_MAIN_CONFIG = 77
CMD_GET_MAIN_CONFIG = 78
TIDSGRANS = 6.0
LOGNAMN_START = 110  # id, kommando, mag/gps/uwb/ap/logg-fälten före loggnamnet


def rcs_slut(conf):
    """Var RControlStations Write slutar (packetinterface.cpp: t.o.m. aktuatorerna).
    Efter loggnamnet: mode+baud 5, fordon 1+4+4, 11 float, 5 float, aktuatorer 2+32."""
    namn_slut = conf.index(0, LOGNAMN_START) + 1
    return namn_slut + 5 + 9 + 44 + 20 + 34


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
    ut = []
    while buf:
        if buf[0] == 2 and len(buf) >= 2:
            n, h = buf[1], 2
        elif buf[0] == 3 and len(buf) >= 3:
            n, h = (buf[1] << 8) | buf[2], 3
        elif buf[0] in (2, 3):
            break
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


class Kort:
    def __init__(self, host):
        self.s = socket.create_connection((host, PORT), timeout=TIDSGRANS)
        self.buf = b""

    def vanta_pa(self, cmd):
        slut = time.time() + TIDSGRANS
        while time.time() < slut:
            self.s.settimeout(max(0.1, slut - time.time()))
            try:
                d = self.s.recv(4096)
            except socket.timeout:
                break
            if not d:
                raise SystemExit("Car_Client stängde anslutningen. Är RControlStation ansluten? Koppla från den.")
            self.buf += d
            paket, self.buf = plocka_paket(self.buf)
            for p in paket:
                if len(p) >= 2 and p[1] == cmd:
                    return p
        return None

    def las(self):
        self.s.sendall(rama_in(bytes([255, CMD_GET_MAIN_CONFIG])))
        return self.vanta_pa(CMD_GET_MAIN_CONFIG)

    def skriv(self, forsta_delen):
        # Samma som RControlStation: till kortets id, bara t.o.m. aktuatorerna.
        payload = bytes([forsta_delen[0], CMD_SET_MAIN_CONFIG]) + forsta_delen[2:]
        self.s.sendall(rama_in(payload))
        return self.vanta_pa(CMD_SET_MAIN_CONFIG)


def beskriv(conf):
    u16 = lambda o: struct.unpack(">H", conf[o:o + 2])[0]
    f32 = lambda o: struct.unpack(">f", conf[o:o + 4])[0]
    slut = rcs_slut(conf)
    if len(conf) < slut + 36:
        return "kort svar (%d byte)" % len(conf)
    # heartbeat: 4+34 byte före slutet; sensorer direkt efter; loopar efter sensorerna
    return "heartbeat=%g aktuatorer=%d sensorer=%d reglerloopar=%d" % (
        f32(slut - 38), u16(slut - 34), u16(slut), u16(slut + 34))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    antal = int(args[0]) if args else 3
    host = "127.0.0.1"

    k = Kort(host)
    conf = k.las()
    if not conf:
        raise SystemExit("Kortet svarar inte på läsning. Starta om det först.")
    print("Före:   id=%d %s" % (conf[0], beskriv(conf)))
    original = conf[:rcs_slut(conf)]

    for i in range(1, antal + 1):
        kvitto = k.skriv(original)
        conf = k.las()
        if not conf:
            print("Write %d: kortet svarar INTE längre%s. FEL." % (
                i, "" if kvitto else " (inget kvitto heller)"))
            print("Starta om kortet över ST-Link.")
            sys.exit(1)
        print("Write %d: %s, lever. %s" % (i, "kvitterad" if kvitto else "INGET kvitto", beskriv(conf)))
        if conf[:len(original)] != original:
            print("  OBS: inställningarna t.o.m. aktuatorerna skiljer sig från före!")
        time.sleep(1.0)

    print("OK: %d Write i rad utan att kortet hängde." % antal)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""fw_version.py — läser styrkortets firmwareversion via Car_Client (bara läsning).

Skickar CMD_GET_STATE (120) till ID 255 och skriver ut versionen, t.ex. "30.2".
Används av uppdateringsskriptet för att se om kortet behöver flashas.

  python3 fw_version.py              Car_Client på den här datorn (127.0.0.1)
  python3 fw_version.py 192.168.200.12
  python3 fw_version.py --kalla MAPP  versionen i firmware-källan (conf_general.h)

Car_Client tar bara en TCP-klient: RControlStation/robotstyrning ska vara frånkopplade.
Avslutar med kod 1 om kortet inte svarar.
"""
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from las_styrkort import rama_in, plocka_paket  # noqa: E402
import socket  # noqa: E402

CMD_GET_STATE = 120


def las_kortet(host, forsok=6):
    for _ in range(forsok):
        try:
            s = socket.create_connection((host, 8300), timeout=3)
        except OSError:
            time.sleep(2)
            continue
        try:
            s.sendall(rama_in(bytes([255, CMD_GET_STATE])))
            buf = b""
            slut = time.time() + 3
            while time.time() < slut:
                s.settimeout(max(0.1, slut - time.time()))
                try:
                    d = s.recv(4096)
                except socket.timeout:
                    break
                if not d:
                    break  # Car_Client upptagen av en annan klient
                buf += d
                paket, buf = plocka_paket(buf)
                for p in paket:
                    if len(p) >= 4 and p[1] == CMD_GET_STATE:
                        return "%d.%d" % (p[2], p[3])
        finally:
            s.close()
        time.sleep(2)
    return None


def las_kallan(mapp):
    txt = open(os.path.join(mapp, "conf_general.h")).read()
    major = re.search(r"#define FW_VERSION_MAJOR\s+(\d+)", txt).group(1)
    minor = re.search(r"#define FW_VERSION_MINOR\s+(\d+)", txt).group(1)
    return "%s.%s" % (major, minor)


def main():
    if len(sys.argv) >= 3 and sys.argv[1] == "--kalla":
        print(las_kallan(sys.argv[2]))
        return
    host = sys.argv[1] if len(sys.argv) > 1 else "127.0.0.1"
    v = las_kortet(host)
    if not v:
        sys.exit(1)
    print(v)


if __name__ == "__main__":
    main()

#!/bin/bash
# Ge USB-portarna, modemet och VPN-tunneln 90 sekunder att vakna efter boot
# (rättat 2026-09-20 — filen hade `sleep 10`, vilket inte matchade den 90
# sekunder som faktiskt löste startproblemen med RControlStation
# tidigare. Se PROJECT_SPEC.md §19 i robot-control för bakgrund.)
sleep 90

# Startar Car_Client i en bakgrunds-screen
screen -S car -d -m bash -c "cd '/home/robant/RControllStation/rise_sdvp/Linux/Car_Client' && ./Car_Client -p /dev/vehicle --useudp --logusb --usetcp --tcprtcmserver 8200 --tcpubxserver 8210 --setid 4; bash"
echo "Car_Client startades i en screen-session med namnet 'car'."
echo "För att ansluta live, kör: screen -r car"

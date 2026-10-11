#!/bin/sh
section() { echo; echo "=== $1"; }
section "1 system"
cat /proc/version
ldd --version 2>&1 | head -1
ls -l /lib/libc.so.6 /lib/libc-*.so /lib/ld-linux-aarch64.so.1 2>&1
strings /usr/trimui/bin/MainUI 2>/dev/null | grep '^Trimui' | head -3
section "2 input"
cat /proc/bus/input/devices
for d in /sys/class/input/event*; do
	echo "$(basename "$d") name=$(cat "$d/device/name") key=[$(cat "$d/device/capabilities/key")] abs=[$(cat "$d/device/capabilities/abs")] sw=[$(cat "$d/device/capabilities/sw" 2>/dev/null)]"
done
section "3 gpu"
ls /usr/lib /lib 2>/dev/null | grep -i -E 'egl|gles|pvr|mali|gbm|sdl' | sort -u
cat /sys/class/graphics/fb0/modes /sys/class/graphics/fb0/virtual_size /sys/class/graphics/fb0/bits_per_pixel 2>&1
section "4 audio"
cat /proc/asound/cards
amixer scontrols
amixer contents 2>&1 | head -120
ls -l /sys/class/speaker 2>&1
cat /sys/class/speaker/mute 2>&1
section "5 processes (MainUI running)"
ps
section "6 leds"
ls -l /sys/class/led_anim
for f in /sys/class/led_anim/*; do [ -f "$f" ] && echo "$(basename "$f")=$(head -c 120 "$f" 2>&1)"; done
section "7 rumble"
ls /sys/class/motor /sys/class/gpio 2>&1
cat /sys/class/motor/voltage 2>&1
section "8 bios and roms"
ls -la /mnt/SDCARD
for d in /mnt/SDCARD/BIOS /mnt/SDCARD/Bios /mnt/SDCARD/RetroArch/.retroarch/system; do echo "-- $d"; ls "$d" 2>&1 | head -20; done
find /mnt/SDCARD -maxdepth 5 -iname 'gba_bios.bin' 2>/dev/null
ls /mnt/SDCARD/Roms 2>&1 | head -80
ls -la /mnt/SDCARD/Roms/GBA 2>&1 | head -20
section "9 boot"
cat /etc/inittab 2>&1
ls /etc/init.d
grep -rn -E 'starts|MainUI|cmd_to_run' /etc/init.d /usr/trimui/bin/*.sh 2>/dev/null | head -40
for f in $(grep -rl -E 'System/starts' /etc/init.d /usr/trimui/bin 2>/dev/null); do echo "---- $f"; cat "$f"; done
section "10 wifi"
ls /sys/class/net
which wpa_supplicant wpa_cli iw ip rfkill hostapd 2>&1
iw list 2>&1 | sed -n '/Supported interface modes/,/Band 1/p'
iw list 2>&1 | sed -n '/valid interface combinations/,/^$/p'
iw list 2>&1 | grep -E '^\s+\* [0-9]+ MHz' | head -60
ps | grep -i -E 'wpa|udhcpc|hostapd' | grep -v grep
ls /etc/wifi 2>&1
for f in /etc/wifi/*.sh /etc/wifi/*.conf; do [ -f "$f" ] && { echo "---- $f"; head -60 "$f"; }; done
section "11 run dir"
ls -ld /run /tmp /var/run 2>&1
touch /run/slot-probe 2>/dev/null && rm /run/slot-probe && echo run-writable || echo run-not-writable
section "12 mounts"
cat /proc/mounts

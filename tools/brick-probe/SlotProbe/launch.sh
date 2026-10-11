#!/bin/sh
OUT=/mnt/SDCARD/slot-probe
LED=/sys/class/led_anim
rm -rf "$OUT"
mkdir -p "$OUT"
ps > "$OUT/ps.txt" 2>&1
cat /tmp/cmd_to_run.sh >> "$OUT/ps.txt" 2>&1
cat /proc/bus/input/devices > "$OUT/input-devices.txt"
amixer cget name='digital volume' > "$OUT/volume-before.txt" 2>&1
amixer cget name='DAC volume' >> "$OUT/volume-before.txt" 2>&1
for z in m lr; do
	echo 000000 > "$LED/effect_rgb_hex_$z"
	echo -1 > "$LED/effect_cycles_$z"
	echo 4 > "$LED/effect_$z"
done
echo FF0000 > "$LED/effect_rgb_hex_f1"
echo -1 > "$LED/effect_cycles_f1"
echo 4 > "$LED/effect_f1"
echo 00FF00 > "$LED/effect_rgb_hex_f2"
echo -1 > "$LED/effect_cycles_f2"
echo 4 > "$LED/effect_f2"
pids=""
for n in /dev/input/event*; do
	name="$(basename "$n")"
	hexdump -v -e '24/1 "%02x" "\n"' "$n" | sed "s#^#$name #" >> "$OUT/input.log" &
	pids="$pids $!"
done
sleep 60
kill $pids 2>/dev/null
killall hexdump 2>/dev/null
amixer cget name='digital volume' > "$OUT/volume-after.txt" 2>&1
amixer cget name='DAC volume' >> "$OUT/volume-after.txt" 2>&1
echo 3300000 > /sys/class/motor/voltage
for i in 1 2 3; do
	echo 1 > /sys/class/gpio/gpio227/value
	sleep 0.3
	echo 0 > /sys/class/gpio/gpio227/value
	sleep 0.3
done
sync

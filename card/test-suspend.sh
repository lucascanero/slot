#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/Apps/Slot/suspend.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

setup() {
	rm -rf "$TMP/bin" "$TMP"/*.log "$TMP/state"
	mkdir -p "$TMP/bin"
	: > "$TMP/state"
	cat > "$TMP/bin/pgrep" <<-SH
		#!/bin/sh
		exit ${1:-0}
	SH
	cat > "$TMP/bin/killall" <<-SH
		#!/bin/sh
		echo "\$*" >> "$TMP/killall.log"
	SH
	cat > "$TMP/bin/restore" <<-SH
		#!/bin/sh
		echo restored >> "$TMP/restore.log"
	SH
	chmod +x "$TMP/bin/pgrep" "$TMP/bin/killall" "$TMP/bin/restore"
}

run() {
	PATH="$TMP/bin:$PATH" SLOT_POWER_STATE="${STATE:-$TMP/state}" SLOT_SUSPEND_TRIES=2 \
		SLOT_SUSPEND_RETRY_S=0 SLOT_WIFI_RESTORE="$TMP/bin/restore" sh "$SCRIPT"
}

wait_for() {
	i=0
	while [ "$i" -lt 30 ] && [ ! -f "$1" ]; do
		sleep 0.1
		i=$((i + 1))
	done
}

setup
run || { echo "a sleep the kernel took was reported as failed" >&2; exit 1; }
[ "$(cat "$TMP/state")" = "mem" ] || { echo "the kernel was never asked to sleep" >&2; exit 1; }
grep -q wpa_supplicant "$TMP/killall.log" || { echo "WiFi was left up through the sleep" >&2; exit 1; }
grep -q udhcpc "$TMP/killall.log" || { echo "the DHCP client was left running through the sleep" >&2; exit 1; }
wait_for "$TMP/restore.log"
[ -f "$TMP/restore.log" ] || { echo "WiFi did not come back after the sleep" >&2; exit 1; }

setup 1
run
[ ! -f "$TMP/killall.log" ] || { echo "touched a WiFi that was off" >&2; exit 1; }
sleep 0.3
[ ! -f "$TMP/restore.log" ] || { echo "turned on a WiFi that was off before the sleep" >&2; exit 1; }

setup
if STATE="$TMP/missing/state" run; then
	echo "a sleep that never happened was reported as fine" >&2
	exit 1
fi

echo "suspend: all passed"

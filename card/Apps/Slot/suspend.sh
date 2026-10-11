#!/bin/sh
STATE="${SLOT_POWER_STATE:-/sys/power/state}"
TRIES="${SLOT_SUSPEND_TRIES:-5}"
RETRY="${SLOT_SUSPEND_RETRY_S:-3}"
WIFI_RESTORE="${SLOT_WIFI_RESTORE:-wpa_supplicant -B -D nl80211 -iwlan0 -c /etc/wifi/wpa_supplicant.conf -O /etc/wifi/sockets; udhcpc -i wlan0 -b}"

wifi=
if pgrep wpa_supplicant > /dev/null 2>&1; then
	wifi=1
	killall udhcpc > /dev/null 2>&1
	killall wpa_supplicant > /dev/null 2>&1
fi
sync

rc=1
i=0
while [ "$i" -lt "$TRIES" ]; do
	i=$((i + 1))
	slept_at=$(date +%s)
	if { echo mem > "$STATE"; } 2> /dev/null; then
		rc=0
		break
	fi
	if [ $(($(date +%s) - slept_at)) -gt 5 ]; then
		rc=0
		break
	fi
	sleep "$RETRY"
done

if [ -n "$wifi" ]; then
	(sh -c "$WIFI_RESTORE" > /dev/null 2>&1 &)
fi
exit "$rc"

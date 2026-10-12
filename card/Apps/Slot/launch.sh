#!/bin/sh
APP="$(cd "$(dirname "$0")" && pwd)"
SD="${SLOT_SD:-/mnt/SDCARD}"
DATA="$SD/Slot"
REQUEST="$DATA/Config/retroarch.request"
CORE_PATTERN="${SLOT_CORE_PATTERN:-/proc/sys/kernel/core_pattern}"
STOP="${SLOT_STOP-keymon}"
PROC="${SLOT_PROC:-/proc}"
TAB="$(printf '\t')"
mkdir -p "$DATA/Config"
[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
: > "$DATA/slot.log"
rm -f "$REQUEST" "$REQUEST.run"
OLD_CORE="$(cat "$CORE_PATTERN" 2>/dev/null)"
echo "|/bin/sh $APP/coresave.sh %p %s %e %t" > "$CORE_PATTERN" 2>/dev/null
export SLOT_SD="$SD" SLOT_APP="$APP"

STOPPED=""
for p in $STOP; do
	for pid in $(pidof "$p" 2>/dev/null); do
		exe="$(readlink "$PROC/$pid/exe" 2>/dev/null)" || continue
		cwd="$(readlink "$PROC/$pid/cwd" 2>/dev/null)"
		lib="$(tr '\000' '\n' < "$PROC/$pid/environ" 2>/dev/null | sed -n 's/^LD_LIBRARY_PATH=//p')"
		STOPPED="$STOPPED$exe$TAB${cwd:-/}$TAB$lib
"
		break
	done
done

stop_daemons() {
	for p in $STOP; do killall "$p" 2>/dev/null; done
}

start_daemons() {
	printf '%s' "$STOPPED" | while IFS="$TAB" read -r exe cwd lib; do
		[ -n "$exe" ] || continue
		(cd "$cwd" && LD_LIBRARY_PATH="$lib" "$exe" > /dev/null 2>&1 &)
	done
}

stop_daemons
cd "$APP" || exit 1
while :; do
	"$APP/slot" >> "$DATA/slot.log" 2>&1
	[ -f "$REQUEST" ] || break
	mv -f "$REQUEST" "$REQUEST.run"
	link="$(sed -n 's/^link=//p' "$REQUEST.run" | head -n 1)"
	start_daemons
	sh "$APP/retroarch.sh" "$REQUEST.run" > "$DATA/retroarch.log" 2>&1
	rm -f "$REQUEST.run"
	stop_daemons
	[ -n "$link" ] && sh "$APP/slotlink.sh" link down >> "$DATA/slot.log" 2>&1
	cd "$APP" || exit 1
done
[ -n "$OLD_CORE" ] && echo "$OLD_CORE" > "$CORE_PATTERN" 2>/dev/null
exit 0

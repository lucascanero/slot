#!/bin/sh
APP="$(cd "$(dirname "$0")" && pwd)"
SD="${SLOT_SD:-/mnt/SDCARD}"
DATA="$SD/Slot"
CORE_PATTERN="${SLOT_CORE_PATTERN:-/proc/sys/kernel/core_pattern}"
STOP="${SLOT_STOP-keymon}"
mkdir -p "$DATA"
[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
OLD_CORE="$(cat "$CORE_PATTERN" 2>/dev/null)"
echo "|/bin/sh $APP/coresave.sh %p %s %e %t" > "$CORE_PATTERN" 2>/dev/null
export SLOT_SD="$SD" SLOT_APP="$APP"
for p in $STOP; do killall "$p" 2>/dev/null; done
cd "$APP" || exit 1
"$APP/slot" > "$DATA/slot.log" 2>&1
[ -n "$OLD_CORE" ] && echo "$OLD_CORE" > "$CORE_PATTERN" 2>/dev/null
exit 0

#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

mkdir -p "$TMP/sd" "$TMP/app"
cp "$HERE/Apps/Slot/launch.sh" "$TMP/app/launch.sh"
cat > "$TMP/app/slot" <<-SH
	#!/bin/sh
	echo "sd=\$SLOT_SD app=\$SLOT_APP pwd=\$(pwd)"
	echo "pattern=\$(cat "$TMP/core_pattern")"
SH
chmod +x "$TMP/app/slot"
echo "core" > "$TMP/core_pattern"
mkdir -p "$TMP/sd/Slot"
echo "old run" > "$TMP/sd/Slot/slot.log"

mkdir -p "$TMP/bin"
cat > "$TMP/bin/killall" <<-SH
	#!/bin/sh
	echo "\$*" >> "$TMP/killall.log"
SH
chmod +x "$TMP/bin/killall"
cat >> "$TMP/app/slot" <<-SH
	echo "killed=\$(cat "$TMP/killall.log" 2>/dev/null)"
SH

PATH="$TMP/bin:$PATH" SLOT_SD="$TMP/sd" SLOT_CORE_PATTERN="$TMP/core_pattern" sh "$TMP/app/launch.sh"

log="$TMP/sd/Slot/slot.log"
grep -q "sd=$TMP/sd app=$TMP/app pwd=$TMP/app" "$log" \
	|| { echo "slot ran without SLOT_SD and SLOT_APP: [$(cat "$log")]" >&2; exit 1; }
grep -q "pattern=|/bin/sh $TMP/app/coresave.sh %p %s %e %t" "$log" \
	|| { echo "the core pattern was not pointed at coresave.sh while slot ran" >&2; exit 1; }
[ "$(cat "$TMP/core_pattern")" = "core" ] \
	|| { echo "the core pattern was not given back" >&2; exit 1; }
[ "$(cat "$TMP/sd/Slot/slot.log.1")" = "old run" ] \
	|| { echo "the last run's log was not kept" >&2; exit 1; }
grep -q "killed=keymon" "$log" \
	|| { echo "the stock keymon still owned Vol and power while slot ran: [$(cat "$log")]" >&2; exit 1; }
[ "$(grep -c . "$TMP/killall.log")" = "1" ] \
	|| { echo "launch.sh touched keymon again after slot: [$(cat "$TMP/killall.log")]" >&2; exit 1; }

echo "launch: all passed"

#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

REQ="$TMP/sd/Slot/Config/retroarch.request"

setup() {
	rm -rf "$TMP/sd" "$TMP/app" "$TMP/bin" "$TMP/proc" "$TMP"/*.log "$TMP/runs" "$TMP/keymon.up"
	mkdir -p "$TMP/sd/Slot/Config" "$TMP/app" "$TMP/bin" "$TMP/proc/123" "$TMP/keymon-home" "$TMP/keymon-bin"
	cp "$HERE/Apps/Slot/launch.sh" "$TMP/app/launch.sh"
	echo core > "$TMP/core_pattern"

	cat > "$TMP/keymon-bin/keymon" <<-SH
		#!/bin/sh
		echo "keymon cwd=\$(pwd) lib=\$LD_LIBRARY_PATH" >> "$TMP/events.log"
		: > "$TMP/keymon.up"
	SH
	ln -s "$TMP/keymon-bin/keymon" "$TMP/proc/123/exe"
	ln -s "$TMP/keymon-home" "$TMP/proc/123/cwd"
	printf 'HOME=/root\000LD_LIBRARY_PATH=/keymon/lib\000' > "$TMP/proc/123/environ"

	cat > "$TMP/bin/pidof" <<-SH
		#!/bin/sh
		[ "\$1" = keymon ] && echo 123
	SH
	cat > "$TMP/bin/killall" <<-SH
		#!/bin/sh
		echo "killall \$*" >> "$TMP/events.log"
		rm -f "$TMP/keymon.up"
	SH

	cat > "$TMP/app/retroarch.sh" <<-SH
		#!/bin/sh
		i=0
		while [ ! -f "$TMP/keymon.up" ] && [ "\$i" -lt 30 ]; do sleep 0.1; i=\$((i + 1)); done
		[ -f "$TMP/keymon.up" ] && k=up || k=down
		echo "retroarch mode=\$(sed -n 's/^mode=//p' "\$1") keymon=\$k" >> "$TMP/events.log"
		exit \${RA_EXIT:-0}
	SH
	cat > "$TMP/app/slotlink.sh" <<-SH
		#!/bin/sh
		echo "slotlink \$*" >> "$TMP/events.log"
	SH
	chmod +x "$TMP/keymon-bin/keymon" "$TMP/bin/pidof" "$TMP/bin/killall" "$TMP/app/retroarch.sh" "$TMP/app/slotlink.sh"
}

# The fake slot: run N writes line N of $TMP/plan as the request (or nothing for "-").
slot_from_plan() {
	cat > "$TMP/app/slot" <<-SH
		#!/bin/sh
		n=\$((\$(cat "$TMP/runs" 2>/dev/null || echo 0) + 1))
		echo "\$n" > "$TMP/runs"
		[ -f "$REQ" ] && echo "slot \$n found a request" >> "$TMP/events.log"
		echo "slot \$n" >> "$TMP/events.log"
		line="\$(sed -n "\${n}p" "$TMP/plan")"
		[ -n "\$line" ] && [ "\$line" != "-" ] && printf '%b' "\$line" > "$REQ"
		exit 0
	SH
	chmod +x "$TMP/app/slot"
}

run() {
	PATH="$TMP/bin:$PATH" SLOT_SD="$TMP/sd" SLOT_CORE_PATTERN="$TMP/core_pattern" \
		SLOT_PROC="$TMP/proc" sh "$TMP/app/launch.sh"
}

expect() {
	printf '%s\n' "$@" > "$TMP/want.log"
	if ! cmp -s "$TMP/want.log" "$TMP/events.log"; then
		echo "$WHAT" >&2
		diff "$TMP/want.log" "$TMP/events.log" >&2 || true
		exit 1
	fi
}

setup
printf 'mode=game\\ncore=/c.so\\ncore_name=mGBA\\nrom=/r.gba\\nlink=host\\n\n-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a linked game did not go slot -> keymon -> RetroArch -> keymon off -> link down -> slot"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=game keymon=up" \
	"killall keymon" \
	"slotlink link down" \
	"slot 2"
[ ! -e "$REQ" ] && [ ! -e "$REQ.run" ] || { echo "the request was left on the card" >&2; exit 1; }
[ "$(cat "$TMP/core_pattern")" = core ] || { echo "the core pattern was not given back" >&2; exit 1; }

setup
printf 'mode=menu\\n\n-\n' > "$TMP/plan"
slot_from_plan
RA_EXIT=139 run
WHAT="a RetroArch that crashed did not come back to slot, or a menu without a link took the link down"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 2"

setup
printf 'mode=menu\\n' > "$REQ"
printf -- '-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a request left from before this launch opened RetroArch on its own"
expect \
	"killall keymon" \
	"slot 1"

setup
printf 'mode=menu\\n\nmode=menu\\n\n-\n' > "$TMP/plan"
slot_from_plan
run
WHAT="a second trip to RetroArch did not bring keymon back the same way"
expect \
	"killall keymon" \
	"slot 1" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 2" \
	"keymon cwd=$TMP/keymon-home lib=/keymon/lib" \
	"retroarch mode=menu keymon=up" \
	"killall keymon" \
	"slot 3"

setup
mkdir -p "$TMP/sd/trimui/res/skin"
: > "$TMP/sd/trimui/.slot"
printf -- '-\n' > "$TMP/plan"
slot_from_plan
run
[ ! -e "$TMP/sd/trimui" ] || { echo "slot's loading from a session cut short was left for StockMix's games" >&2; exit 1; }

setup
mkdir -p "$TMP/sd/trimui/res/skin"
printf -- '-\n' > "$TMP/plan"
slot_from_plan
run
[ -d "$TMP/sd/trimui/res/skin" ] || { echo "a trimui folder that was not slot's was removed" >&2; exit 1; }

echo "launch-retroarch: all passed"

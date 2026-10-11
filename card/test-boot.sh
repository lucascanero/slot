#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/Apps/Slot/boot/slot-boot.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

setup() {
	rm -rf "$TMP/app" "$TMP/bin" "$TMP"/*.log "$TMP/cmd_to_run.sh"
	mkdir -p "$TMP/app" "$TMP/bin"
	: > "$TMP/app/launch.sh"
	cat > "$TMP/bin/pgrep" <<-SH
		#!/bin/sh
		echo "\$*" >> "$TMP/pgrep.log"
		exit ${1:-0}
	SH
	cat > "$TMP/bin/killall" <<-SH
		#!/bin/sh
		echo "\$*" >> "$TMP/killall.log"
	SH
	chmod +x "$TMP/bin/pgrep" "$TMP/bin/killall"
}

run() {
	PATH="$TMP/bin:$PATH" SLOT_APP="$TMP/app" SLOT_CMD_TO_RUN="$TMP/cmd_to_run.sh" \
		SLOT_BOOT_WAIT=3 sh "$SCRIPT"
	i=0
	while [ "$i" -lt 30 ] && [ ! -f "$TMP/killall.log" ]; do
		sleep 0.1
		i=$((i + 1))
	done
	sleep 0.2
}

setup
run
grep -q 'cd "'"$TMP"'/app"' "$TMP/cmd_to_run.sh" \
	|| { echo "the hand-off does not start from the app: [$(cat "$TMP/cmd_to_run.sh" 2>/dev/null)]" >&2; exit 1; }
grep -q 'sh ./launch.sh' "$TMP/cmd_to_run.sh" \
	|| { echo "the hand-off does not run launch.sh" >&2; exit 1; }
[ "$(cat "$TMP/killall.log")" = "-9 MainUI" ] \
	|| { echo "MainUI was not closed to run the hand-off: [$(cat "$TMP/killall.log")]" >&2; exit 1; }
grep -q -- '-x MainUI' "$TMP/pgrep.log" \
	|| { echo "the hook did not wait for MainUI by name" >&2; exit 1; }

setup
rm "$TMP/app/launch.sh"
run
[ ! -f "$TMP/cmd_to_run.sh" ] || { echo "a hook with no app still handed off" >&2; exit 1; }
[ ! -f "$TMP/killall.log" ] || { echo "a hook with no app closed MainUI" >&2; exit 1; }

setup
cat > "$TMP/bin/pgrep" <<-SH
	#!/bin/sh
	exit 1
SH
chmod +x "$TMP/bin/pgrep"
run
sleep 0.5
[ ! -f "$TMP/cmd_to_run.sh" ] || { echo "the hook handed off with no MainUI to replace" >&2; exit 1; }
[ ! -f "$TMP/killall.log" ] || { echo "the hook killed a MainUI that never came" >&2; exit 1; }

echo "boot: all passed"

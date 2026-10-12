#!/bin/sh
set -eu

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT="$HERE/Apps/Slot/retroarch.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT HUP INT TERM

RA="$TMP/sd/RetroArch"
CFG="$RA/.retroarch/config"
GAME="Pokémon - Emerald (U) [!] & Co's"
ROM="$TMP/sd/Roms/GBA/$GAME.gba"
CORE="$TMP/app/mgba_libretro.so"

setup() {
	rm -rf "$TMP/sd" "$TMP/app" "$TMP"/*.log "$TMP/ra_hotkey"
	mkdir -p "$RA/.retroarch" "$TMP/sd/Roms/GBA" "$TMP/app" "$TMP/sd/System/usr/trimui/scripts"
	: > "$ROM"
	: > "$CORE"
	echo 'rgui_config_directory = "~/.retroarch/config"' > "$RA/.retroarch/retroarch.cfg"
	cat > "$RA/ra64.trimui" <<-SH
		#!/bin/sh
		{
			printf 'args='; printf '%s|' "\$@"; echo
			echo "home=\$HOME pwd=\$(pwd)"
			echo "lib=\$LD_LIBRARY_PATH"
			[ -f "$TMP/ra_hotkey" ] && echo "hotkey=yes" || echo "hotkey=no"
			cat "$CFG/mGBA/$GAME.opt" 2>/dev/null | sed 's/^/opt: /'
		} >> "$TMP/ra.log"
		exit \${RA_EXIT:-0}
	SH
	for s in ra_audio_switcher.sh cpufreq.sh; do
		cat > "$TMP/sd/System/usr/trimui/scripts/$s" <<-SH
			#!/bin/sh
			echo "$s \$*" >> "$TMP/env.log"
		SH
	done
	chmod +x "$RA/ra64.trimui" "$TMP/sd/System/usr/trimui/scripts/"*.sh
}

request() {
	printf '%s\n' "$@" > "$TMP/request"
}

run() {
	SLOT_SD="$TMP/sd" SLOT_RA_HOTKEY="$TMP/ra_hotkey" sh "$SCRIPT" "$TMP/request"
}

fail() {
	echo "$1" >&2
	[ -f "$TMP/ra.log" ] && sed 's/^/  ra: /' "$TMP/ra.log" >&2
	exit 1
}

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=
run || fail "a plain game did not run"
grep -qxF "args=-v|-L|$CORE|$ROM|" "$TMP/ra.log" || fail "a plain game got the wrong arguments"
grep -qxF "home=$RA/ pwd=$RA" "$TMP/ra.log" || fail "RetroArch did not run from its own folder with HOME set"
grep -q "^lib=/usr/trimui/lib:$TMP/sd/System/lib" "$TMP/ra.log" || fail "the StockMix libraries are not first"
grep -qx "hotkey=yes" "$TMP/ra.log" || fail "RetroArch ran without its hotkeys"
[ ! -e "$TMP/ra_hotkey" ] || fail "the hotkey file slot made was left behind for Slot's MENU"
grep -qx "ra_audio_switcher.sh " "$TMP/env.log" || fail "the audio switcher did not run"
grep -qx "cpufreq.sh ondemand 2 6" "$TMP/env.log" || fail "the CPU was not set like a StockMix game"

setup
: > "$TMP/ra_hotkey"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host
run
grep -qxF "args=-v|-L|$CORE|--host|$ROM|" "$TMP/ra.log" || fail "host did not ask RetroArch to host"
[ -e "$TMP/ra_hotkey" ] || fail "a hotkey file that was already there was taken away"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=join
run
grep -qxF "args=-v|-L|$CORE|--connect|10.42.0.1|$ROM|" "$TMP/ra.log" || fail "join did not connect to the host"

setup
request mode=menu
run
grep -qxF "args=-v|" "$TMP/ra.log" || fail "the menu was not opened without a game"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host option=mgba_link=on option=mgba_link_player=0
run
grep -qxF 'opt: mgba_link = "on"' "$TMP/ra.log" || fail "the link option was not there while RetroArch ran"
grep -qxF 'opt: mgba_link_player = "0"' "$TMP/ra.log" || fail "the player was not there while RetroArch ran"
[ ! -e "$CFG/mGBA/$GAME.opt" ] || fail "the session's options were left for the next game"

setup
mkdir -p "$CFG/mGBA"
printf 'mgba_link = "off"\nmgba_skip_bios = "ON"\n' > "$CFG/mGBA/$GAME.opt"
cp "$CFG/mGBA/$GAME.opt" "$TMP/mine.opt"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=join option=mgba_link=on option=mgba_link_player=1
run
grep -qxF 'opt: mgba_skip_bios = "ON"' "$TMP/ra.log" || fail "the user's own option was dropped for the session"
grep -qxF 'opt: mgba_link = "off"' "$TMP/ra.log" && fail "the user's link setting won over the session's"
cmp -s "$CFG/mGBA/$GAME.opt" "$TMP/mine.opt" || fail "the user's options file was not given back"

setup
mkdir -p "$CFG/mGBA"
printf 'mgba_link = "on"\n' > "$CFG/mGBA/$GAME.opt"
: > "$CFG/mGBA/$GAME.opt.none"
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=host option=mgba_link=on option=mgba_link_player=0
run
[ ! -e "$CFG/mGBA/$GAME.opt" ] || fail "a session cut short by a power loss left its options behind for good"
[ ! -e "$CFG/mGBA/$GAME.opt.none" ] || fail "the marker of a cut session was left behind"

setup
request mode=game "core=$CORE" core_name=mGBA "rom=$ROM" link=
RA_EXIT=3 run && fail "a RetroArch that failed was reported as fine"
RA_EXIT=3 run || [ $? -eq 3 ] || fail "RetroArch's own exit code was not passed on"

for bad in "mode=bogus" "mode=game core=$CORE" "mode=game rom=$ROM" "mode=game core=$TMP/none.so rom=$ROM"; do
	setup
	# shellcheck disable=SC2086
	request $bad
	if run 2> /dev/null; then
		fail "[$bad] was taken as a request"
	fi
	[ ! -f "$TMP/ra.log" ] || fail "RetroArch ran for [$bad]"
done

setup
rm -f "$RA/ra64.trimui"
request mode=menu
if run 2> /dev/null; then
	fail "a card with no RetroArch reported success"
fi

echo "retroarch: all passed"

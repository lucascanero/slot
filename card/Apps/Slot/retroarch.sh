#!/bin/sh
REQUEST="$1"
SD="${SLOT_SD:-/mnt/SDCARD}"
RA_DIR="${SLOT_RA_DIR:-$SD/RetroArch}"
SCRIPTS="${SLOT_RA_SCRIPTS:-$SD/System/usr/trimui/scripts}"
HOTKEY="${SLOT_RA_HOTKEY:-/var/trimui_inputd/ra_hotkey}"
LINK_HOST="${SLOT_LINK_HOST:-10.42.0.1}"

value() {
	sed -n "s/^$1=//p" "$REQUEST" 2> /dev/null | head -n 1
}

mode="$(value mode)"
core="$(value core)"
core_name="$(value core_name)"
rom="$(value rom)"
link="$(value link)"

case "$mode" in
game)
	if [ ! -f "$core" ] || [ ! -f "$rom" ]; then
		echo "retroarch.sh: no core or no rom in $REQUEST" >&2
		exit 2
	fi
	;;
menu) ;;
*)
	echo "retroarch.sh: $REQUEST is not a request" >&2
	exit 2
	;;
esac
if [ ! -x "$RA_DIR/ra64.trimui" ]; then
	echo "retroarch.sh: no RetroArch at $RA_DIR" >&2
	exit 2
fi

export PATH="$SCRIPTS:$SD/System/bin:$PATH:$RA_DIR"
export LD_LIBRARY_PATH="/usr/trimui/lib:$SD/System/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

config_dir() {
	dir="$(sed -n 's/^rgui_config_directory = "\(.*\)"$/\1/p' "$RA_DIR/.retroarch/retroarch.cfg" 2> /dev/null | head -n 1)"
	case "$dir" in
	"" | default) echo "$RA_DIR/.retroarch/config" ;;
	"~/"*) echo "$RA_DIR/${dir#\~/}" ;;
	":/"*) echo "$RA_DIR/${dir#:/}" ;;
	"./"*) echo "$RA_DIR/${dir#./}" ;;
	/*) echo "$dir" ;;
	*) echo "$RA_DIR/$dir" ;;
	esac
}

opt=""
if [ "$mode" = game ] && [ -n "$link" ] && grep -q '^option=' "$REQUEST"; then
	game="$(basename "$rom")"
	opt_dir="$(config_dir)/$core_name"
	opt="$opt_dir/${game%.*}.opt"
	mkdir -p "$opt_dir"
	if [ ! -f "$opt.slot" ] && [ ! -f "$opt.none" ]; then
		if [ -f "$opt" ]; then
			cp -f "$opt" "$opt.slot"
		else
			: > "$opt.none"
		fi
	fi
	if [ -f "$opt.slot" ]; then
		cp -f "$opt.slot" "$opt.new"
	elif [ -f "$opt_dir/$core_name.opt" ]; then
		cp -f "$opt_dir/$core_name.opt" "$opt.new"
	else
		: > "$opt.new"
	fi
	sed -n 's/^option=//p' "$REQUEST" | while IFS='=' read -r key val; do
		grep -v "^$key = " "$opt.new" > "$opt.key"
		mv -f "$opt.key" "$opt.new"
		printf '%s = "%s"\n' "$key" "$val" >> "$opt.new"
	done
	mv -f "$opt.new" "$opt"
fi

hotkey_made=""
if [ ! -e "$HOTKEY" ] && touch "$HOTKEY" 2> /dev/null; then
	hotkey_made=1
fi
command -v ra_audio_switcher.sh > /dev/null 2>&1 && ra_audio_switcher.sh
command -v cpufreq.sh > /dev/null 2>&1 && cpufreq.sh ondemand 2 6

set --
case "$link" in
host) set -- --host ;;
join) set -- --connect "$LINK_HOST" ;;
esac
cd "$RA_DIR" || exit 2
if [ "$mode" = game ]; then
	HOME="$RA_DIR/" ./ra64.trimui -v -L "$core" "$@" "$rom"
else
	HOME="$RA_DIR/" ./ra64.trimui -v
fi
rc=$?

if [ -n "$opt" ]; then
	if [ -f "$opt.slot" ]; then
		mv -f "$opt.slot" "$opt"
	else
		rm -f "$opt"
	fi
	rm -f "$opt.none"
fi
[ -n "$hotkey_made" ] && rm -f "$HOTKEY"
exit "$rc"

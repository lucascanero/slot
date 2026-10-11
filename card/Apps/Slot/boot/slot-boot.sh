#!/bin/sh
APP="${SLOT_APP:-/mnt/SDCARD/Apps/Slot}"
CMD="${SLOT_CMD_TO_RUN:-/tmp/cmd_to_run.sh}"
WAIT="${SLOT_BOOT_WAIT:-600}"
[ -f "$APP/launch.sh" ] || exit 0
(
	i=0
	while [ "$i" -lt "$WAIT" ] && ! pgrep -x MainUI > /dev/null 2>&1; do
		sleep 0.1
		i=$((i + 1))
	done
	pgrep -x MainUI > /dev/null 2>&1 || exit 0
	printf '#!/bin/sh\ncd "%s" || exit 0\nsh ./launch.sh\n' "$APP" > "$CMD"
	killall -9 MainUI
) > /dev/null 2>&1 &
exit 0

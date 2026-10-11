#!/bin/sh
set -eu
bin="$1" floor="$2"
need="$(readelf -V --wide "$bin" | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/GLIBC_//' | sort -V | tail -1)"
if [ -z "$need" ]; then
	echo "$bin: no glibc versions referenced"
	exit 0
fi
top="$(printf '%s\n%s\n' "$need" "$floor" | sort -V | tail -1)"
[ "$top" = "$floor" ] || { echo "$bin needs glibc $need, the Brick has $floor" >&2; exit 1; }
echo "$bin: glibc $need <= $floor"

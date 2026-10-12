"""Check that a built aarch64 gpSP flushes freshly emitted dynarec code safely.

The dynarec writes ARM64 blocks and then calls platform_cache_sync. On a
Cortex-A53 that must be: clean the D-cache to the point of unification (dc cvau),
dsb ish, invalidate the I-cache (ic ivau), dsb ish, isb. zig's compiler-rt
__clear_cache leaves out the second dsb ish, so the Brick ran stale bytes and
died with SIGILL. This reads platform_cache_sync out of the shared object and
checks the sequence is there, in order, in the function itself.
"""

import struct
import subprocess
import sys

DC_CVAU = (0xFFFFFFE0, 0xD50B7B20)
IC_IVAU = (0xFFFFFFE0, 0xD50B7520)
DSB_ISH = (0xFFFFFFFF, 0xD5033B9F)
ISB = (0xFFFFF0FF, 0xD50330DF)
RET = (0xFFFFFFFF, 0xD65F03C0)
B = (0xFC000000, 0x14000000)


def is_(word, op):
    mask, value = op
    return word & mask == value


def symbol(path, name):
    out = subprocess.run(["nm", path], capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[2] == name:
            return int(parts[0], 16)
    return None


def text_section(path):
    out = subprocess.run(["readelf", "-S", "-W", path], capture_output=True, text=True, check=True).stdout
    for line in out.splitlines():
        parts = line.replace("[ ", "[").split()
        if ".text" in parts:
            i = parts.index(".text")
            return int(parts[i + 2], 16), int(parts[i + 3], 16)
    raise SystemExit(f"{path}: no .text section")


def body(path, addr, limit=128):
    taddr, toff = text_section(path)
    data = open(path, "rb").read()
    off = addr - taddr + toff
    words = []
    for i in range(limit):
        w = struct.unpack_from("<I", data, off + 4 * i)[0]
        words.append(w)
        if is_(w, RET) or is_(w, B):
            break
    return words


def check(path):
    addr = symbol(path, "platform_cache_sync")
    if addr is None:
        return "no platform_cache_sync symbol"
    words = body(path, addr)
    steps = []
    for w in words:
        for name, op in (("dc cvau", DC_CVAU), ("dsb ish", DSB_ISH), ("ic ivau", IC_IVAU), ("isb", ISB)):
            if is_(w, op) and (not steps or steps[-1] != name):
                steps.append(name)
    want = ["dc cvau", "dsb ish", "ic ivau", "dsb ish", "isb"]
    if steps != want:
        tail = " (it hands off with a branch)" if words and is_(words[-1], B) else ""
        return f"platform_cache_sync runs {steps or 'no cache maintenance'}{tail}, want {want}"
    return None


if __name__ == "__main__":
    problem = check(sys.argv[1])
    if problem:
        print(f"FAIL: {sys.argv[1]}: {problem}")
        sys.exit(1)
    print(f"ok: {sys.argv[1]}: platform_cache_sync cleans, waits, invalidates, waits and synchronises")

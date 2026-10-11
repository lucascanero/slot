import sys

for line in sys.stdin:
    parts = line.split()
    if len(parts) != 2:
        continue
    node, hexbytes = parts
    b = bytes.fromhex(hexbytes)
    if len(b) != 24:
        continue
    t = int.from_bytes(b[16:18], "little")
    c = int.from_bytes(b[18:20], "little")
    v = int.from_bytes(b[20:24], "little", signed=True)
    if t == 0:
        continue
    print(f"{node} type={t:#04x} code={c:#05x} ({c}) value={v}")

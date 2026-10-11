# Slot para TrimUI Brick (StockMix): plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pôr o Slot (frontend Rust de GB/GBC/GBA) a correr no TrimUI Brick com StockMix, como app em `Apps/` com opção de arrancar no Slot, mantendo todas as funcionalidades do original.

**Architecture:** É um fork só para Brick do repositório original, no branch `brick`. Mudam três coisas:
- **Camada de hardware:** input do gamepad TrimUI, `/dev/disp`, `led_anim`, rumble por GPIO e mixer ALSA, por trás dos traits que já existem (`Platform`, `InputSource`).
- **Layout de pastas:** um `Layout` instalado uma vez no arranque. Quando não há nenhum, o código volta ao layout original, por isso os testes existentes continuam a passar.
- **Pacote para o cartão SD:** `Apps/Slot` com os seus scripts.

A build para o dispositivo usa `cargo-zigbuild` e `zig cc` no WSL, fixando a glibc a 2.31.

**Tech Stack:** Rust 1.96 (workspace com 8 crates), `libloading` (EGL/GLES/ALSA por `dlopen`), `libc`, scripts POSIX `sh` (busybox no Brick), cmake/make para os cores libretro (mGBA, gpSP), `zig` 0.14.1, `cargo-zigbuild`, `go-task`, WSL Ubuntu 26.04.

**Spec:** [docs/superpowers/specs/2026-10-11-slot-brick-port-design.md](../specs/2026-10-11-slot-brick-port-design.md)

## Global Constraints

- Toolchain Rust: o de `rust-toolchain.toml` (`channel = "1.96"`), edição 2021. Sem dependências novas no `Cargo.toml` do workspace.
- Alvo do dispositivo: `aarch64-unknown-linux-gnu`, glibc ≤ a do Brick. Por omissão é **2.31**; a Tarefa 2 confirma, e se o Brick tiver menos, baixa-se `BRICK_GLIBC` no `taskfile.yml`.
- Caminhos no Brick:
  - app em `/mnt/SDCARD/Apps/Slot`
  - dados em `/mnt/SDCARD/Slot`
  - ROMs em `/mnt/SDCARD/Roms/{GBA,GB,GBC}`
- O Slot **nunca** escreve em `Roms/` nem no eMMC ou na partição de arranque.
- LEDs: só os dos botões F1/F2 acendem; a barra superior (`m`) e as laterais (`lr`) ficam a preto enquanto o Slot corre. Tudo é reposto ao sair.
- Cores dos LEDs:

  | Estado | Cor |
  |---|---|
  | A correr / carregado | `00FF00` |
  | A carregar | `FF8C00` |
  | Bateria fraca | `FF0000` |
  | Desligado | `000000` |

- Brilho: passo 0 → 0 (só para o doze); passos 1..9 → `[8, 16, 32, 48, 72, 96, 128, 176, 255]`.
- Ordem do menu de energia: Exit to Menu, Restart, Power Off.
- O "Start on Boot" fica no fim da página Gameplay e o seu estado é a existência do ficheiro de arranque.
- Finais de linha LF em tudo. O repositório tem `core.autocrlf=false`; não mudar.
- Estilo: igual ao código existente.
  - Quase sem comentários.
  - Nomes de testes em frase (`a_thing_does_what_it_should`).
  - Mensagens de commit em inglês, no imperativo, curtas (como `Add 34XX and 34XXSP`), terminadas com a linha `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`.
- **Cada tarefa termina com tudo verde, corrido no WSL a partir de `/mnt/c/Users/lucas/Slot`:**

  ```sh
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo clippy -p slot --no-default-features --features device -- -D warnings
  cargo test --workspace
  sh card/test-slotlink.sh && sh card/test-coresave.sh   # e card/test-launch.sh a partir da Tarefa 10
  ```

  Exceção: falhas que já existiam na linha de base registada na Tarefa 1 não contam.

## Review Focus

1. **Pasta `Roms/` stock com lixo.** Tem `.zip`, a cache do MainUI (`GBA_cache7.db`) e subpastas. O Slot deve mostrar só as ROMs soltas, sem falhar. O teste está na Tarefa 4.
2. **Cartão sem `gba_bios.bin` em lado nenhum.** O Slot deve arrancar na mesma, usando `Slot/BIOS` como pasta de sistema (BIOS HLE). O teste está na Tarefa 4.
3. **"Start on Boot" com a app mal instalada** (falta `boot/slot-boot.sh`). O menu não pode mentir: fica Off e nada rebenta. Os testes estão na Tarefa 12.
4. **Gatilhos L2/R2 cujo intervalo não se consegue ler** (o `ioctl` falha ou o máximo é 0). Qualquer valor acima de 0 conta como premido e 0 solta. O teste está na Tarefa 5.
5. **"Exit to Menu" escolhido a meio de um jogo.** O ponto de retoma tem de ficar gravado no cartão antes de sair, para reabrir no mesmo sítio. O teste está na Tarefa 11.

---

## Estrutura de ficheiros

| Ficheiro | Estado | Responsabilidade |
|---|---|---|
| `crates/slot-store/src/layout.rs` | novo | `Layout` (pastas StockMix / flat), `app_dir()` |
| `crates/slot-store/src/boot_hook.rs` | novo | instalar e remover o script de "Start on Boot" |
| `crates/slot-store/src/scan.rs` | alterado | `scan_in(&Layout)`; `scan(root)` delega |
| `crates/slot/src/root.rs` | alterado | `DATA_DIRS`, `ensure_layout`, BIOS pelo `Layout` |
| `crates/slot/src/core.rs`, `bezel.rs`, `link_radio.rs` | alterados | caminhos da app pelo `Layout` |
| `crates/slot/src/main.rs`, `host_app.rs`, `device_app.rs` | alterados | `--init-sd`, `SLOT_SD`/`SLOT_APP`, ligação ao Brick, Exit |
| `crates/slot/src/input/evdev.rs` | alterado | mapa TrimUI, `Trigger`/`Sticks`, `Picked` e seleção de nós sem duplicados |
| `crates/slot/src/input/device.rs` | alterado | leitura filtrada por nó, flag dos auscultadores, `EVIOCGABS`/`EVIOCGSW`; sem tampa |
| `crates/slot/src/input/trace.rs` | alterado | trace com auscultadores, L2/R2 e eixos |
| `crates/slot-power/src/device.rs` | reescrito | `DevicePlatform` do Brick: `/dev/disp`, bateria, RTC, motor GPIO, LEDs, `release()` |
| `crates/slot-power/src/mixer.rs` | novo | `Amixer`, `CliAmixer`, `HwVolume` (guardar, pôr no máximo, repor) |
| `crates/slot-power/src/lib.rs`, `power.rs`, `sim.rs` | alterados | `release()` no trait; sem `relink_adb` |
| `crates/slot-ui/src/power_menu.rs` | alterado | `PowerChoice::Exit` |
| `crates/slot-ui/src/quick_menu.rs` | alterado | `QuickRow::StartOnBoot` |
| `crates/slot/src/app.rs`, `frontend.rs` | alterados | Exit, Start on Boot |
| `card/Apps/Slot/*` | novo (move `card/System/*`) | `config.json`, `launch.sh`, `icon.png`, `coresave.sh`, `slotlink.sh`, `boot/slot-boot.sh`, `bezels/` |
| `card/test-launch.sh` | novo | testa o `launch.sh` fora do dispositivo |
| `tools/brick-probe/*` | novo | sonda de hardware do Brick |
| `tools/zig/*`, `tools/glibc-floor.sh` | novos | cross-compilação e verificação da glibc |
| `cores/mgba/build.sh`, `cores/gpsp/build.sh` | alterados | modo cross (`SLOT_CROSS_*`) |
| `taskfile.yml` | alterado | tarefas Brick |
| `docs/brick-dev.md`, `docs/brick-hardware.md` | novos | setup e resultados da sonda |

---

### Task 1: Toolchain no WSL e linha de base

**Files:**
- Create: `docs/brick-dev.md`

**Interfaces:**
- Consumes: nada
- Produces: ambiente WSL com `cargo` 1.96, target `aarch64-unknown-linux-gnu`, `zig` 0.14.1, `cargo-zigbuild`, `task`, `cmake`, `imagemagick`, `librsvg2-bin`; lista de falhas da linha de base em `docs/brick-dev.md`

O utilizador pode ter de escrever a password do `sudo` do WSL. Se o comando ficar à espera, pede-lhe que o corra no terminal dele.

- [ ] **Step 1: Instalar pacotes de sistema no WSL**

```sh
wsl -d Ubuntu-26.04 -- sudo apt-get update
wsl -d Ubuntu-26.04 -- sudo apt-get install -y build-essential pkg-config libasound2-dev libudev-dev \
  cmake git curl xz-utils unzip file python3 openssh-client imagemagick librsvg2-bin binutils
```

Esperado: termina sem erros.

- [ ] **Step 2: Instalar rustup e o toolchain fixado**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none'
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && rustup show && rustup target add aarch64-unknown-linux-gnu && cargo --version'
```

Esperado: `cargo 1.96.x`.

- [ ] **Step 3: Instalar zig 0.14.1, cargo-zigbuild e task**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'mkdir -p ~/.local/opt ~/.local/bin && cd ~/.local/opt && curl -fsSLO https://ziglang.org/download/0.14.1/zig-x86_64-linux-0.14.1.tar.xz && tar -xf zig-x86_64-linux-0.14.1.tar.xz && ln -sf ~/.local/opt/zig-x86_64-linux-0.14.1/zig ~/.local/bin/zig && zig version'
wsl -d Ubuntu-26.04 -- bash -lc 'cargo install cargo-zigbuild --locked && sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b ~/.local/bin && task --version'
```

Esperado: `0.14.1`, depois a versão do `task`.
- Se o URL do zig der 404, abrir `https://ziglang.org/download/index.json`, procurar a entrada `0.14.1` → `x86_64-linux` → `tarball` e usar esse URL.
- Garantir que `~/.local/bin` está no `PATH` do `bash -l` (o Ubuntu acrescenta-o se a pasta existir no login).

- [ ] **Step 4: Correr a linha de base, sem alterações**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && cargo test --workspace 2>&1 | tail -40; sh card/test-slotlink.sh; sh card/test-coresave.sh; sh card/test-led.sh'
```

Esperado: tudo passa, ou uma lista curta de falhas. Copiar para o Step 5 o resumo de cada `test result:` com `FAILED` e o nome dos testes que falharam.

- [ ] **Step 5: Escrever `docs/brick-dev.md`**

```markdown
# Desenvolvimento do Slot para TrimUI Brick

Tudo corre no WSL (`wsl -d Ubuntu-26.04`), a partir de `/mnt/c/Users/lucas/Slot`.

## Ferramentas

- Rust: o toolchain de `rust-toolchain.toml` mais `rustup target add aarch64-unknown-linux-gnu`
- zig 0.14.1 em `~/.local/bin/zig`
- `cargo install cargo-zigbuild --locked`
- go-task em `~/.local/bin/task`
- apt: build-essential pkg-config libasound2-dev libudev-dev cmake imagemagick librsvg2-bin binutils openssh-client

## Linha de base (antes do port, commit 9d7c795)

<colar aqui o resultado do Step 4: "todos os testes passam", ou a lista de testes que já falhavam>
```

Substituir a linha entre `< >` pelo resultado real.

- [ ] **Step 6: Commit**

```bash
git add docs/brick-dev.md
git commit -m "docs: WSL toolchain for the Brick port and the test baseline

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: Sonda ao Brick

**Files:**
- Create: `tools/brick-probe/survey.sh`, `tools/brick-probe/decode.py`, `tools/brick-probe/SlotProbe/config.json`, `tools/brick-probe/SlotProbe/launch.sh`, `docs/brick-hardware.md`

**Interfaces:**
- Consumes: SSH ao Brick (BrickTools: utilizador `root`, password `tina`), com IP dado pelo utilizador
- Produces: `docs/brick-hardware.md`, cujas secções §1–§13 as tarefas seguintes citam

Esta tarefa precisa do utilizador: o IP do Brick, abrir a app de sonda e carregar em botões.

- [ ] **Step 1: Escrever `tools/brick-probe/survey.sh` (corre no Brick)**

```sh
#!/bin/sh
section() { echo; echo "=== $1"; }
section "1 system"
cat /proc/version
ldd --version 2>&1 | head -1
ls -l /lib/libc.so.6 /lib/libc-*.so /lib/ld-linux-aarch64.so.1 2>&1
strings /usr/trimui/bin/MainUI 2>/dev/null | grep '^Trimui' | head -3
section "2 input"
cat /proc/bus/input/devices
for d in /sys/class/input/event*; do
	echo "$(basename "$d") name=$(cat "$d/device/name") key=[$(cat "$d/device/capabilities/key")] abs=[$(cat "$d/device/capabilities/abs")] sw=[$(cat "$d/device/capabilities/sw" 2>/dev/null)]"
done
section "3 gpu"
ls /usr/lib /lib 2>/dev/null | grep -i -E 'egl|gles|pvr|mali|gbm|sdl' | sort -u
cat /sys/class/graphics/fb0/modes /sys/class/graphics/fb0/virtual_size /sys/class/graphics/fb0/bits_per_pixel 2>&1
section "4 audio"
cat /proc/asound/cards
amixer scontrols
amixer contents 2>&1 | head -120
ls -l /sys/class/speaker 2>&1
cat /sys/class/speaker/mute 2>&1
section "5 processes (MainUI running)"
ps
section "6 leds"
ls -l /sys/class/led_anim
for f in /sys/class/led_anim/*; do [ -f "$f" ] && echo "$(basename "$f")=$(head -c 120 "$f" 2>&1)"; done
section "7 rumble"
ls /sys/class/motor /sys/class/gpio 2>&1
cat /sys/class/motor/voltage 2>&1
section "8 bios and roms"
ls -la /mnt/SDCARD
for d in /mnt/SDCARD/BIOS /mnt/SDCARD/Bios /mnt/SDCARD/RetroArch/.retroarch/system; do echo "-- $d"; ls "$d" 2>&1 | head -20; done
find /mnt/SDCARD -maxdepth 5 -iname 'gba_bios.bin' 2>/dev/null
ls /mnt/SDCARD/Roms 2>&1 | head -80
ls -la /mnt/SDCARD/Roms/GBA 2>&1 | head -20
section "9 boot"
cat /etc/inittab 2>&1
ls /etc/init.d
grep -rn -E 'starts|MainUI|cmd_to_run' /etc/init.d /usr/trimui/bin/*.sh 2>/dev/null | head -40
for f in $(grep -rl -E 'System/starts' /etc/init.d /usr/trimui/bin 2>/dev/null); do echo "---- $f"; cat "$f"; done
section "10 wifi"
ls /sys/class/net
which wpa_supplicant wpa_cli iw ip rfkill hostapd 2>&1
iw list 2>&1 | sed -n '/Supported interface modes/,/Band 1/p'
iw list 2>&1 | sed -n '/valid interface combinations/,/^$/p'
iw list 2>&1 | grep -E '^\s+\* [0-9]+ MHz' | head -60
ps | grep -i -E 'wpa|udhcpc|hostapd' | grep -v grep
ls /etc/wifi 2>&1
for f in /etc/wifi/*.sh /etc/wifi/*.conf; do [ -f "$f" ] && { echo "---- $f"; head -60 "$f"; }; done
section "11 run dir"
ls -ld /run /tmp /var/run 2>&1
touch /run/slot-probe 2>/dev/null && rm /run/slot-probe && echo run-writable || echo run-not-writable
section "12 mounts"
cat /proc/mounts
```

- [ ] **Step 2: Escrever a app de sonda (corre como app, com o MainUI fechado)**

`tools/brick-probe/SlotProbe/config.json`:
```json
{
    "label": "SlotProbe",
    "icon": "",
    "launch": "launch.sh",
    "description": "Slot hardware probe (60 s)"
}
```

`tools/brick-probe/SlotProbe/launch.sh`:
```sh
#!/bin/sh
OUT=/mnt/SDCARD/slot-probe
rm -rf "$OUT"
mkdir -p "$OUT"
ps > "$OUT/ps.txt" 2>&1
cat /tmp/cmd_to_run.sh >> "$OUT/ps.txt" 2>&1
cat /proc/bus/input/devices > "$OUT/input-devices.txt"
pids=""
for n in /dev/input/event*; do
	name="$(basename "$n")"
	hexdump -v -e '24/1 "%02x" "\n"' "$n" | sed "s#^#$name #" >> "$OUT/input.log" &
	pids="$pids $!"
done
sleep 60
kill $pids 2>/dev/null
killall hexdump 2>/dev/null
sync
```

- [ ] **Step 3: Escrever `tools/brick-probe/decode.py` (corre no PC)**

```python
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
```

- [ ] **Step 4: Pedir ao utilizador o acesso**

Pedir:
1. Ligar o SSH no BrickTools (Network → SSH Server).
2. Dizer o IP (BrickTools → Network → IP Address).

Daqui em diante, `BRICK=root@<ip>` e a password é `tina`.

Testar com:
```sh
wsl -d Ubuntu-26.04 -- bash -lc 'ssh -o StrictHostKeyChecking=accept-new root@<ip> "echo ok"'
```

- [ ] **Step 5: Correr o survey e instalar a app de sonda**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && ssh root@<ip> sh -s < tools/brick-probe/survey.sh > /tmp/brick-survey.txt 2>&1; wc -l /tmp/brick-survey.txt'
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot/tools/brick-probe && tar -cf - SlotProbe | ssh root@<ip> "tar -C /mnt/SDCARD/Apps -xf - && chmod +x /mnt/SDCARD/Apps/SlotProbe/launch.sh && sync"'
```

- [ ] **Step 6: O utilizador corre a sonda de botões**

Dar estas instruções ao utilizador:
1. Abre **Apps → SlotProbe**. O ecrã fica parado; isso é normal.
2. Espera 3 s e carrega em cada um destes, com cerca de 1 s entre cada:
   A, B, X, Y, L1, R1, L2, R2, Select, Start, Menu, F1, F2, Vol+, Vol−, cima, baixo, esquerda, direita.
3. Liga e desliga os auscultadores.
4. No fim, um toque curto no power.
5. Ao fim de 60 s volta ao MainUI.

Depois:
```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && ssh root@<ip> cat /mnt/SDCARD/slot-probe/input.log | python3 tools/brick-probe/decode.py > /tmp/brick-input.txt; ssh root@<ip> cat /mnt/SDCARD/slot-probe/ps.txt > /tmp/brick-ps.txt; head -80 /tmp/brick-input.txt'
```

O `ps.txt` mostra que processos ficam vivos enquanto uma app corre (§5). O `input.log` mostra o código real de cada botão (§2).

- [ ] **Step 7: Testes ao vivo de LEDs e rumble (o utilizador confirma o que vê)**

```sh
ssh root@<ip> 'cd /sys/class/led_anim && echo FF0000 > effect_rgb_hex_f1 && echo -1 > effect_cycles_f1 && echo 4 > effect_f1'
```
Perguntar: "o botão F1 ficou vermelho fixo?"

```sh
ssh root@<ip> 'cd /sys/class/led_anim && echo 000000 > effect_rgb_hex_m && echo 4 > effect_m && echo 000000 > effect_rgb_hex_lr && echo 4 > effect_lr'
```
Perguntar: "a barra de cima e as laterais apagaram, e o F1 continua aceso?"

```sh
ssh root@<ip> '[ -d /sys/class/gpio/gpio227 ] || echo 227 > /sys/class/gpio/export; echo out > /sys/class/gpio/gpio227/direction; echo 3300000 > /sys/class/motor/voltage; echo 1 > /sys/class/gpio/gpio227/value; sleep 0.3; echo 0 > /sys/class/gpio/gpio227/value'
```
Perguntar: "vibrou?"

No fim, pedir ao utilizador para reiniciar o Brick, o que repõe os LEDs.

- [ ] **Step 8: Escrever `docs/brick-hardware.md` com os resultados**

Usar exatamente estas secções, preenchidas com o que os Steps 5–7 devolveram (excertos literais, não resumos):

```markdown
# TrimUI Brick: resultados da sonda (StockMix)

## §1 Sistema e glibc
<versão do kernel, linha do ldd, modelo>
**glibc do Brick:** <x.yy>  →  BRICK_GLIBC a usar: <2.31 ou menor>

## §2 Input
<linhas "eventN name=... key=[...] abs=[...] sw=[...]">
Tabela observada (de /tmp/brick-input.txt):
| Botão | nó | type | code |
|---|---|---|---|
| A | | | |
| B | | | |
| X | | | |
| Y | | | |
| L1 | | | |
| R1 | | | |
| L2 | | | |
| R2 | | | |
| Select | | | |
| Start | | | |
| Menu | | | |
| F1 | | | |
| F2 | | | |
| Vol+ | | | |
| Vol− | | | |
| D-pad | | | |
| Power | | | |
| Auscultadores | | | |
Nome do gamepad virtual: <ex.: "TRIMUI Player1">
Nós que repetem botões do gamepad: <lista>

## §3 GPU e ecrã
<bibliotecas EGL/GLES/SDL encontradas; modo do fb0>

## §4 Áudio
<cartas; controlos de amixer relevantes com valores atuais; /sys/class/speaker/mute>

## §5 Processos enquanto uma app corre
<ps.txt; o MainUI morre? que daemons ficam (trimui_inputd, keymon, ...)? algum reage ao power ou ao volume?>
<o toque no power durante a sonda pôs o Brick a dormir? sim/não>

## §6 LEDs
<lista de ficheiros e valores; formato lido de effect_rgb_hex_*; resultado do teste ao vivo>

## §7 Rumble
<gpio227 existe/exportado? motor/voltage? vibrou?>

## §8 BIOS e ROMs
<pastas de BIOS existentes; onde está gba_bios.bin; nomes exatos das pastas em Roms/>

## §9 Arranque
<script que corre System/starts/*.sh; corre em primeiro plano ou em fundo? antes ou depois do MainUI? o trimui_inputd já corre nessa altura?>

## §10 WiFi
<interfaces; wlan1 existe?; modos suportados (AP?); combinações AP+managed; frequências AP possíveis; ctrl_interface do wpa_supplicant stock; comando stock que liga o WiFi>

## §11 /run
<run-writable ou run-not-writable>

## §12 Mounts
<linha de /mnt/SDCARD>

## §13 Decisões que daqui resultam
- Mapa de botões: <igual ao previsto na Tarefa 5 | diferenças: ...>
- BRICK_GLIBC: <valor>
- Mecanismo de "Start on Boot": <pasta e modo>
- Link por WiFi: interface AP <wlan1 | criar com iw | wlan0>, frequência <5745 | 2437>, ctrl stock <dir>, restauro <comando>
- Daemons a pausar durante o Slot: <nenhum | lista>
```

- [ ] **Step 9: Mostrar a §13 ao utilizador antes de continuar**

Se algo mudar o design, por exemplo "o chip não suporta AP", dizê-lo já, numa frase, com a consequência. Não continuar para a Tarefa 3 sem resposta nesse caso.

- [ ] **Step 10: Commit**

```bash
git add tools/brick-probe docs/brick-hardware.md
git commit -m "Probe the Brick's input, LEDs, rumble, audio, boot and WiFi

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Remover o que é só BaseOS/Anbernic

**Files:**
- Delete: `crates/slot/src/bootlogo.rs`, `crates/slot/tests/bootlogo.rs`, `card/System/bootlogo.bmp`, `card/System/launch_frontend.sh`, `card/System/led.sh`, `card/test-led.sh`, `site/`, `CNAME`, `.github/workflows/pages.yml`
- Modify: `crates/slot/src/lib.rs`, `crates/slot/src/device_app.rs`, `crates/slot-power/src/lib.rs`, `crates/slot-power/src/power.rs`, `crates/slot-power/src/sim.rs`, `crates/slot-power/src/device.rs`, `crates/slot-power/tests/device.rs`, `taskfile.yml`

**Interfaces:**
- Consumes: nada
- Produces: o trait `Platform` sem `relink_adb`; `SimPlatform` sem `relinks()`; `device_app::run` sem boot logo e sem rotação

- [ ] **Step 1: Apagar ficheiros**

```bash
git rm -r -q crates/slot/src/bootlogo.rs crates/slot/tests/bootlogo.rs card/System/bootlogo.bmp \
  card/System/launch_frontend.sh card/System/led.sh card/test-led.sh site CNAME .github/workflows/pages.yml
```

- [ ] **Step 2: Tirar o módulo `bootlogo`**

Em `crates/slot/src/lib.rs`, apagar a linha `pub mod bootlogo;`.

Em `crates/slot/src/device_app.rs`, apagar o bloco:
```rust
    let card = root.clone();
    let _ = std::thread::Builder::new()
        .name("slot-bootlogo".into())
        .spawn(move || slot::bootlogo::refresh(&card));
```

- [ ] **Step 3: Tirar a rotação de `device_app.rs`**

Substituir o bloco desde `let panel = surface.window_size();` até ao fecho de `if turned { ... }` por:
```rust
    let window = surface.window_size();
    eprintln!("slot: panel {}x{}", window.0, window.1);
```

- [ ] **Step 4: Tirar `relink_adb`**

- `crates/slot-power/src/lib.rs`: apagar o método por omissão `fn relink_adb(&mut self) -> bool { false }` do trait `Platform`.
- `crates/slot-power/src/power.rs`: apagar `pub fn relink_adb(&mut self) -> bool { ... }`.
- `crates/slot-power/src/sim.rs`:
  - apagar o campo `relinks`, a sua inicialização em `at()`, o método `relinks()` e o `fn relink_adb` do `impl Platform`;
  - apagar também as linhas `use std::sync::...` que deixarem de ser usadas.
- `crates/slot-power/src/device.rs`: apagar o `fn relink_adb(&mut self) -> bool { ... }` do `impl Platform for DevicePlatform`.
- `crates/slot-power/tests/device.rs`: apagar os testes `relinking_adb_rebinds_the_gadget_to_the_controller` e `relinking_adb_without_a_gadget_does_nothing`.

- [ ] **Step 5: Confirmar que nada mais os usa**

```sh
grep -rn "bootlogo\|relink\|SLOT_ROTATE\|set_panel_turn" crates/slot/src crates/slot-power crates/slot/tests taskfile.yml
```

Esperado: só `crates/slot-gfx/src/fbo.rs` (o código de rotação que fica parado) e, no `taskfile.yml`, a linha `sh card/test-led.sh`. Apagar essa linha de `test:card`.

- [ ] **Step 6: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde. `card/test-led.sh` já não existe.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Drop the BaseOS boot logo, ADB relink, panel turn and docs site

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: `Layout`: pastas do StockMix

**Files:**
- Create: `crates/slot-store/src/layout.rs`, `crates/slot-store/tests/layout.rs`, `crates/slot-store/tests/layout_install.rs`
- Modify: `crates/slot-store/src/lib.rs`, `crates/slot-store/src/scan.rs`, `crates/slot/src/root.rs`, `crates/slot/src/core.rs:18-33`, `crates/slot/src/bezel.rs`, `crates/slot/tests/bezel.rs:1-7`, `crates/slot/src/link_radio.rs:91-104,185-192`, `crates/slot/src/main.rs`, `crates/slot/src/host_app.rs:24`, `crates/slot/tests/dist.rs`

**Interfaces:**
- Consumes: `slot_store::Platform::dir_name()`
- Produces:
  - `slot_store::Layout`, com:
    - `flat(&Path) -> Layout`
    - `stockmix(sd: &Path, app: &Path) -> Layout`
    - `install(self) -> bool`
    - `installed() -> Option<&'static Layout>`
    - `for_root(&Path) -> Layout`
    - `is_flat(&self) -> bool`
    - `data(&self) -> &Path`
    - `roms(&self, Platform) -> PathBuf`
    - `app(&self) -> &Path`
    - `bios_dir(&self) -> PathBuf`
    - `boot_hook(&self) -> &Path`
    - `boot_script(&self) -> PathBuf`
  - `slot_store::app_dir() -> PathBuf`
  - `slot_store::scan_in(&Layout) -> Result<Vec<Cart>, StoreError>`
  - `slot::root::DATA_DIRS: [&str; 15]`, `slot::root::ensure_layout(&Layout)`
  - `slot::bezel::bezel_dir(&Path) -> PathBuf`
  - flag `--init-sd DIR` do binário

- [ ] **Step 1: Escrever os testes do layout**

`crates/slot-store/tests/layout.rs`:
```rust
use std::fs;
use std::path::{Path, PathBuf};

use slot_store::{scan_in, Layout, Platform};

#[test]
fn stockmix_keeps_the_users_data_in_its_own_folder() {
    let l = Layout::stockmix(Path::new("/mnt/SDCARD"), Path::new("/mnt/SDCARD/Apps/Slot"));
    assert_eq!(l.data(), Path::new("/mnt/SDCARD/Slot"));
    assert_eq!(l.roms(Platform::Gba), Path::new("/mnt/SDCARD/Roms/GBA"));
    assert_eq!(l.roms(Platform::Gb), Path::new("/mnt/SDCARD/Roms/GB"));
    assert_eq!(l.roms(Platform::Gbc), Path::new("/mnt/SDCARD/Roms/GBC"));
    assert_eq!(l.app(), Path::new("/mnt/SDCARD/Apps/Slot"));
    assert_eq!(
        l.boot_hook(),
        Path::new("/mnt/SDCARD/System/starts/slot-boot.sh")
    );
    assert_eq!(
        l.boot_script(),
        Path::new("/mnt/SDCARD/Apps/Slot/boot/slot-boot.sh")
    );
    assert!(!l.is_flat());
}

#[test]
fn flat_is_the_layout_slot_always_had() {
    let l = Layout::flat(Path::new("/card"));
    assert_eq!(l.data(), Path::new("/card"));
    assert_eq!(l.roms(Platform::Gba), Path::new("/card/Games/GBA"));
    assert_eq!(l.app(), Path::new("/card/System"));
    assert_eq!(l.bios_dir(), Path::new("/card/BIOS"));
    assert!(l.is_flat());
}

#[test]
fn the_bios_comes_from_the_first_folder_that_holds_one() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    assert_eq!(
        l.bios_dir(),
        d.path().join("Slot/BIOS"),
        "with no bios anywhere, slot's own folder"
    );
    let retroarch = d.path().join("RetroArch/.retroarch/system");
    fs::create_dir_all(&retroarch).unwrap();
    fs::write(retroarch.join("gba_bios.bin"), b"x").unwrap();
    assert_eq!(l.bios_dir(), retroarch);
    fs::create_dir_all(d.path().join("BIOS")).unwrap();
    fs::write(d.path().join("BIOS/gba_bios.bin"), b"x").unwrap();
    assert_eq!(l.bios_dir(), d.path().join("BIOS"));
}

#[test]
fn a_root_with_nothing_installed_is_flat() {
    assert_eq!(
        Layout::for_root(Path::new("/nowhere")),
        Layout::flat(Path::new("/nowhere"))
    );
}

#[test]
fn the_stock_rom_folders_feed_the_shelf_and_nothing_else_does() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    let gba = d.path().join("Roms/GBA");
    fs::create_dir_all(gba.join("Hacks")).unwrap();
    fs::write(gba.join("Emerald.gba"), vec![0u8; 0x100]).unwrap();
    fs::write(gba.join("Zipped.zip"), b"PK").unwrap();
    fs::write(gba.join("GBA_cache7.db"), b"sqlite").unwrap();
    fs::write(gba.join("Hacks/Deep.gba"), vec![0u8; 0x100]).unwrap();
    fs::create_dir_all(d.path().join("Roms/GB")).unwrap();
    fs::write(d.path().join("Roms/GB/Tetris.gb"), vec![0u8; 0x150]).unwrap();
    let mut roms: Vec<PathBuf> = scan_in(&l).unwrap().into_iter().map(|c| c.rom).collect();
    roms.sort();
    let mut want = vec![gba.join("Emerald.gba"), d.path().join("Roms/GB/Tetris.gb")];
    want.sort();
    assert_eq!(roms, want);
}
```

`crates/slot-store/tests/layout_install.rs`. Fica num ficheiro próprio porque é o único teste que instala um layout no processo:
```rust
use std::path::Path;

use slot_store::Layout;

#[test]
fn the_installed_layout_answers_for_its_own_root_only() {
    let l = Layout::stockmix(Path::new("/sd"), Path::new("/sd/Apps/Slot"));
    assert!(l.clone().install());
    assert_eq!(Layout::for_root(Path::new("/sd/Slot")), l);
    assert_eq!(Layout::installed(), Some(&l));
    assert_eq!(
        Layout::for_root(Path::new("/elsewhere")),
        Layout::flat(Path::new("/elsewhere"))
    );
    assert!(
        !Layout::flat(Path::new("/x")).install(),
        "a second install is refused"
    );
}
```

`slot-store` precisa de `tempfile` em `[dev-dependencies]`. Confirmar com `grep -n tempfile crates/slot-store/Cargo.toml`; se faltar, acrescentar `tempfile = { workspace = true }` debaixo de `[dev-dependencies]`.

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot-store --test layout --test layout_install
```

Esperado: erro de compilação `unresolved imports slot_store::scan_in, slot_store::Layout`.

- [ ] **Step 3: Implementar `crates/slot-store/src/layout.rs`**

```rust
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::platform::Platform;

pub const DATA_DIR: &str = "Slot";
pub const BIOS_FILE: &str = "gba_bios.bin";
pub const BOOT_HOOK_DIR: &str = "System/starts";
pub const BOOT_HOOK_NAME: &str = "slot-boot.sh";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    data: PathBuf,
    roms: PathBuf,
    app: PathBuf,
    bios: Vec<PathBuf>,
    boot_hook: PathBuf,
    flat: bool,
}

static INSTALLED: OnceLock<Layout> = OnceLock::new();

impl Layout {
    pub fn flat(root: &Path) -> Layout {
        Layout {
            data: root.to_path_buf(),
            roms: root.join("Games"),
            app: root.join("System"),
            bios: vec![root.join("BIOS")],
            boot_hook: root.join(BOOT_HOOK_DIR).join(BOOT_HOOK_NAME),
            flat: true,
        }
    }

    pub fn stockmix(sd: &Path, app: &Path) -> Layout {
        let data = sd.join(DATA_DIR);
        Layout {
            roms: sd.join("Roms"),
            app: app.to_path_buf(),
            bios: vec![
                sd.join("BIOS"),
                sd.join("Bios"),
                sd.join("RetroArch/.retroarch/system"),
                data.join("BIOS"),
            ],
            boot_hook: sd.join(BOOT_HOOK_DIR).join(BOOT_HOOK_NAME),
            data,
            flat: false,
        }
    }

    pub fn install(self) -> bool {
        INSTALLED.set(self).is_ok()
    }

    pub fn installed() -> Option<&'static Layout> {
        INSTALLED.get()
    }

    pub fn for_root(root: &Path) -> Layout {
        INSTALLED
            .get()
            .filter(|l| l.data() == root)
            .cloned()
            .unwrap_or_else(|| Layout::flat(root))
    }

    pub fn is_flat(&self) -> bool {
        self.flat
    }

    pub fn data(&self) -> &Path {
        &self.data
    }

    pub fn roms(&self, platform: Platform) -> PathBuf {
        self.roms.join(platform.dir_name())
    }

    pub fn app(&self) -> &Path {
        &self.app
    }

    pub fn bios_dir(&self) -> PathBuf {
        self.bios
            .iter()
            .find(|dir| dir.join(BIOS_FILE).is_file())
            .or(self.bios.last())
            .cloned()
            .unwrap_or_else(|| self.data.join("BIOS"))
    }

    pub fn boot_hook(&self) -> &Path {
        &self.boot_hook
    }

    pub fn boot_script(&self) -> PathBuf {
        self.app.join("boot").join(BOOT_HOOK_NAME)
    }
}

pub fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SLOT_APP") {
        return PathBuf::from(dir);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}
```

Em `crates/slot-store/src/lib.rs`, acrescentar `mod layout;` (por ordem alfabética, depois de `mod gba;`) e:
```rust
pub use layout::{app_dir, Layout, BIOS_FILE, BOOT_HOOK_DIR, BOOT_HOOK_NAME, DATA_DIR};
```
Na linha `pub use scan::{...}`, acrescentar `scan_in`.

- [ ] **Step 4: `scan_in` em `crates/slot-store/src/scan.rs`**

Substituir o início de `pub fn scan` até à linha `let labels_dir = ...` (inclusive) por:
```rust
pub fn scan(root: &Path) -> Result<Vec<Cart>, StoreError> {
    scan_in(&crate::layout::Layout::for_root(root))
}

pub fn scan_in(layout: &crate::layout::Layout) -> Result<Vec<Cart>, StoreError> {
    let root = layout.data();
    let read = |file: &str| std::fs::read_to_string(root.join(file)).unwrap_or_default();
    let shells = crate::cart_shell::layered(
        &read(crate::CART_SHELL_FILE),
        &read(crate::LABELS_SHELL_FILE),
    );
    let mut carts = Vec::new();
    for platform in Platform::ALL {
        let dir = layout.roms(platform);
        let entries = match std::fs::read_dir(&dir) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                eprintln!("slot: scan: {}: {e}", dir.display());
                continue;
            }
        };
        let labels_dir = root.join("Labels").join(platform.dir_name());
```
O resto da função fica igual.

- [ ] **Step 5: Correr os testes do `slot-store`**

```sh
cargo test -p slot-store
```

Esperado: PASS, incluindo os testes antigos de `scan`.

- [ ] **Step 6: Escrever o teste do `ensure` para StockMix**

Acrescentar a `crates/slot/tests/dist.rs`:
```rust
#[test]
fn a_stockmix_card_gets_the_data_folders_and_no_rom_or_system_folders() {
    let d = tempdir().unwrap();
    let layout = slot_store::Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    slot::root::ensure_layout(&layout);
    for name in slot::root::DATA_DIRS {
        assert!(
            d.path().join("Slot").join(name).is_dir(),
            "{name} was not created"
        );
    }
    assert!(!d.path().join("Roms").exists(), "ensure made a Roms folder");
    assert!(!d.path().join("Slot/Games").exists());
    assert!(!d.path().join("Slot/System").exists());
}
```

```sh
cargo test -p slot --test dist
```

Esperado: erro de compilação `cannot find ... ensure_layout / DATA_DIRS`.

- [ ] **Step 7: Implementar em `crates/slot/src/root.rs`**

Acrescentar `use slot_store::Layout;` e, depois de `DIRS`:
```rust
pub const DATA_DIRS: [&str; 15] = [
    "BIOS",
    "Config",
    "Labels",
    "Labels/GBA",
    "Labels/GB",
    "Labels/GBC",
    "Saves",
    "Saves/GBA",
    "Saves/GB",
    "Saves/GBC",
    "States",
    "States/GBA",
    "States/GB",
    "States/GBC",
    "Wallpapers",
];
```

Substituir `ensure` e `bios_dir` por:
```rust
pub fn ensure(root: &Path) {
    ensure_layout(&Layout::for_root(root));
}

pub fn ensure_layout(layout: &Layout) {
    let root = layout.data();
    let dirs: &[&str] = if layout.is_flat() { &DIRS } else { &DATA_DIRS };
    for sub in dirs {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    if let Err(e) = slot_store::move_config(root) {
        eprintln!("slot: config: {e}");
    }
}

pub fn bios_dir(root: &Path) -> PathBuf {
    Layout::for_root(root).bios_dir()
}
```

- [ ] **Step 8: Cores, molduras e link pelo `Layout`**

`crates/slot/src/core.rs`, dentro de `candidates`, substituir:
```rust
    let mut paths = vec![root.join("System").join(&name)];
```
por:
```rust
    let mut paths = vec![slot_store::Layout::for_root(root).app().join(&name)];
```

`crates/slot/src/bezel.rs`: trocar `pub const BEZELS: &str = "System/bezels";` por:
```rust
pub const BEZELS: &str = "bezels";

pub fn bezel_dir(root: &Path) -> PathBuf {
    slot_store::Layout::for_root(root).app().join(BEZELS)
}
```
Em `pick`, trocar `std::fs::read_dir(root.join(BEZELS))` por `std::fs::read_dir(bezel_dir(root))`.

`crates/slot/tests/bezel.rs`:
- Linha 1: `use slot::bezel::{bezel_dir, pick};`.
- Nas linhas 5 e 7, trocar `d.path().join(BEZELS)` por `bezel_dir(d.path())`.

`crates/slot/src/link_radio.rs`, substituir `link_command` e `link`:
```rust
#[cfg(any(feature = "device", test))]
fn link_command(app: &Path, verb: &str) -> Command {
    let mut cmd = Command::new("/bin/sh");
    cmd.arg(app.join("slotlink.sh")).arg("link").arg(verb);
    cmd
}

#[cfg(feature = "device")]
fn link(verb: &str) -> Command {
    let app = slot_store::Layout::installed()
        .map(|l| l.app().to_path_buf())
        .unwrap_or_else(slot_store::app_dir);
    link_command(&app, verb)
}
```
No teste `a_link_runs_the_cards_script_through_sh`, usar:
```rust
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("System");
        let script = app.join("slotlink.sh");
        assert_eq!(
            argv(&link_command(&app, "host")),
            ["/bin/sh", script.to_str().unwrap(), "link", "host"]
        );
```

- [ ] **Step 9: `--init-sd` e `SLOT_SD` no modo PC**

Em `crates/slot/src/main.rs`, substituir o `if args.next().as_deref() == Some("--init-root") { ... }` por:
```rust
    let flag = args.next();
    if let Some(flag @ ("--init-root" | "--init-sd")) = flag.as_deref() {
        let Some(dir) = args.next() else {
            eprintln!("slot: {flag} needs a directory");
            std::process::exit(2);
        };
        let dir = std::path::Path::new(&dir);
        match flag {
            "--init-sd" => slot::root::ensure_layout(&slot_store::Layout::stockmix(
                dir,
                &dir.join("Apps/Slot"),
            )),
            _ => slot::root::ensure(dir),
        }
        return;
    }
```

Em `crates/slot/src/host_app.rs`, trocar `Frontend::boot(Box::new(SimPlatform::new()))` por `Frontend::boot(Box::new(host_platform()))` e acrescentar no fim do ficheiro:
```rust
fn host_platform() -> SimPlatform {
    let Some(sd) = std::env::var_os("SLOT_SD") else {
        return SimPlatform::new();
    };
    let layout =
        slot_store::Layout::stockmix(std::path::Path::new(&sd), &slot_store::app_dir());
    let root = layout.data().to_path_buf();
    layout.install();
    SimPlatform::at(root)
}
```

- [ ] **Step 10: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde. Os testes antigos usam o layout flat e continuam iguais.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "Read roms, bios and the app folder through a Layout, with a StockMix one

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Botões do TrimUI e gatilhos L2/R2

**Files:**
- Modify: `crates/slot/src/input/evdev.rs`, `crates/slot/src/input/device.rs` (só `read_node`), `crates/slot/src/input/trace.rs` (só `event_line` e os `use`), `crates/slot/tests/evdev.rs`

**Interfaces:**
- Consumes: `slot_input::{Btn, RawEvent}`, `libc::ioctl`
- Produces:
  - `evdev::code_to_btn` com o mapa TrimUI e `evdev::WANTED: [u16; 12]`
  - `evdev::Trigger::new(Btn, i32)` e `Trigger::feed(i32) -> Option<RawEvent>`
  - `evdev::Sticks::new(i32, i32)` e `Sticks::feed(Ev) -> Vec<RawEvent>`
  - `evdev::{ABS_Z, ABS_RZ, SW_HEADPHONE_INSERT}`
  - `device::abs_max(&File, u16) -> Option<i32>`
  - `to_raw` deixa de mapear `SW_LID`

**Antes de começar:** abrir `docs/brick-hardware.md` §2. A tabela do Step 1 é a prevista (a partir do NextUI). **Se a sonda mostrou outros códigos, os da sonda mandam**: mudar o teste e o código para os valores observados.

- [ ] **Step 1: Escrever os testes**

Em `crates/slot/tests/evdev.rs`:
- Trocar `const CODE_R1: u16 = 0x135;` por `const CODE_R1: u16 = 0x137;`.
- Substituir `the_lid_arrives_as_a_switch_rather_than_a_key` por:
```rust
#[test]
fn a_lid_switch_is_not_a_button_on_the_brick() {
    assert_eq!(decode(&packet(EV_SW, SW_LID, 1)).and_then(to_raw), None);
    assert_eq!(decode(&packet(EV_SW, SW_LID, 0)).and_then(to_raw), None);
}
```
- Substituir o corpo de `the_pad_maps_by_the_codes_the_device_sends` por:
```rust
    let expected = [
        (0x131, Btn::A),
        (0x130, Btn::B),
        (0x134, Btn::X),
        (0x133, Btn::Y),
        (0x136, Btn::L1),
        (0x137, Btn::R1),
        (0x13a, Btn::Select),
        (0x13b, Btn::Start),
        (0x13c, Btn::Menu),
        (115, Btn::VolUp),
        (114, Btn::VolDown),
        (116, Btn::Power),
    ];
    for (code, want) in expected {
        assert_eq!(
            slot::input::evdev::code_to_btn(code),
            Some(want),
            "code {code:#x}"
        );
    }
```
- Acrescentar:
```rust
#[test]
fn f1_and_f2_do_nothing() {
    assert_eq!(slot::input::evdev::code_to_btn(0x13d), None);
    assert_eq!(slot::input::evdev::code_to_btn(0x13e), None);
}

#[test]
fn a_trigger_presses_past_half_and_lets_go_under_a_quarter() {
    let mut t = slot::input::evdev::Trigger::new(Btn::L2, 255);
    assert_eq!(t.feed(100), None);
    assert_eq!(t.feed(130), Some(RawEvent::Down(Btn::L2)));
    assert_eq!(t.feed(200), None);
    assert_eq!(t.feed(70), None, "between a quarter and a half it holds");
    assert_eq!(t.feed(60), Some(RawEvent::Up(Btn::L2)));
    assert_eq!(t.feed(0), None);
}

#[test]
fn a_trigger_with_no_known_range_is_pressed_by_anything_above_zero() {
    let mut t = slot::input::evdev::Trigger::new(Btn::R2, 0);
    assert_eq!(t.feed(1), Some(RawEvent::Down(Btn::R2)));
    assert_eq!(t.feed(255), None);
    assert_eq!(t.feed(0), Some(RawEvent::Up(Btn::R2)));
}

#[test]
fn the_brick_triggers_arrive_on_z_and_rz_beside_the_hat() {
    use slot::input::evdev::{Ev, Sticks, ABS_RZ, ABS_Z, EV_ABS};
    let mut s = Sticks::new(255, 255);
    let ev = |code, value| Ev {
        kind: EV_ABS,
        code,
        value,
    };
    assert_eq!(s.feed(ev(ABS_Z, 255)), vec![RawEvent::Down(Btn::L2)]);
    assert_eq!(s.feed(ev(ABS_RZ, 255)), vec![RawEvent::Down(Btn::R2)]);
    assert_eq!(s.feed(ev(0x11, -1)), vec![RawEvent::Down(Btn::Up)]);
    assert_eq!(s.feed(ev(ABS_Z, 0)), vec![RawEvent::Up(Btn::L2)]);
    assert!(
        s.feed(ev(0x00, 500)).is_empty(),
        "the left stick is not a button"
    );
}

#[test]
fn the_triggers_and_the_jack_are_traced_by_name() {
    use slot::input::evdev::{Ev, ABS_Z, EV_ABS};
    let l2 = slot::input::trace::event_line(
        "event3",
        5,
        Ev {
            kind: EV_ABS,
            code: ABS_Z,
            value: 255,
        },
    );
    assert!(l2.contains("l2"), "{l2}");
    let jack = slot::input::trace::event_line(
        "event2",
        6,
        Ev {
            kind: EV_SW,
            code: 0x02,
            value: 1,
        },
    );
    assert!(jack.contains("headphones"), "{jack}");
}
```

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot --test evdev
```

Esperado: falha de compilação (`Trigger`, `Sticks`, `ABS_Z` não existem).

- [ ] **Step 3: Implementar em `crates/slot/src/input/evdev.rs`**

Junto às outras constantes, acrescentar:
```rust
pub const SW_HEADPHONE_INSERT: u16 = 0x02;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RZ: u16 = 0x05;
```

Substituir `to_raw`, `code_to_btn` e `WANTED` por:
```rust
pub fn to_raw(ev: Ev) -> Option<RawEvent> {
    if ev.kind != EV_KEY {
        return None;
    }
    let btn = code_to_btn(ev.code)?;
    match ev.value {
        0 => Some(RawEvent::Up(btn)),
        1 => Some(RawEvent::Down(btn)),
        _ => None,
    }
}

pub fn code_to_btn(code: u16) -> Option<Btn> {
    Some(match code {
        0x131 => Btn::A,
        0x130 => Btn::B,
        0x134 => Btn::X,
        0x133 => Btn::Y,
        0x136 => Btn::L1,
        0x137 => Btn::R1,
        0x13a => Btn::Select,
        0x13b => Btn::Start,
        0x13c => Btn::Menu,
        115 => Btn::VolUp,
        114 => Btn::VolDown,
        116 => Btn::Power,
        _ => return None,
    })
}

pub const WANTED: [u16; 12] = [
    0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 115, 114, 116,
];
```
(`SW_LID` e `wanted_node` ficam até à Tarefa 6.)

Depois de `impl Hat`, acrescentar:
```rust
pub struct Trigger {
    btn: Btn,
    max: i64,
    held: bool,
}

impl Trigger {
    pub fn new(btn: Btn, max: i32) -> Self {
        Trigger {
            btn,
            max: i64::from(max.max(1)),
            held: false,
        }
    }

    pub fn feed(&mut self, value: i32) -> Option<RawEvent> {
        let v = i64::from(value);
        if !self.held && v * 2 > self.max {
            self.held = true;
            return Some(RawEvent::Down(self.btn));
        }
        if self.held && v * 4 < self.max {
            self.held = false;
            return Some(RawEvent::Up(self.btn));
        }
        None
    }
}

pub struct Sticks {
    hat: Hat,
    l2: Trigger,
    r2: Trigger,
}

impl Sticks {
    pub fn new(l2_max: i32, r2_max: i32) -> Self {
        Sticks {
            hat: Hat::default(),
            l2: Trigger::new(Btn::L2, l2_max),
            r2: Trigger::new(Btn::R2, r2_max),
        }
    }

    pub fn feed(&mut self, ev: Ev) -> Vec<RawEvent> {
        match ev.code {
            ABS_Z => self.l2.feed(ev.value).into_iter().collect(),
            ABS_RZ => self.r2.feed(ev.value).into_iter().collect(),
            _ => self.hat.feed(ev),
        }
    }
}
```

- [ ] **Step 4: Ler o intervalo dos gatilhos em `crates/slot/src/input/device.rs`**

No topo, acrescentar:
```rust
use std::os::unix::io::AsRawFd;

const EVIOCGABS_BASE: libc::c_ulong = 0x8018_4540;

pub fn abs_max(file: &File, code: u16) -> Option<i32> {
    let mut info = [0i32; 6];
    let request = EVIOCGABS_BASE + libc::c_ulong::from(code);
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), request as _, info.as_mut_ptr()) };
    (rc == 0 && info[2] > 0).then_some(info[2])
}
```

Trocar o `use super::evdev::{...}` para importar `Sticks, ABS_RZ, ABS_Z` em vez de `Hat`.

Em `read_node`:
- Trocar `let mut hat = Hat::default();` pela linha abaixo, depois de o ficheiro estar aberto (o `let mut file = match File::open(node)` tem de vir antes):
```rust
    let mut sticks = Sticks::new(
        abs_max(&file, ABS_Z).unwrap_or(1),
        abs_max(&file, ABS_RZ).unwrap_or(1),
    );
```
- Trocar `EV_ABS => events.extend(hat.feed(ev)),` por `EV_ABS => events.extend(sticks.feed(ev)),`.

- [ ] **Step 5: Nomes no trace em `crates/slot/src/input/trace.rs`**

- No `use super::evdev::{...}`, trocar `SW_LID` por `SW_HEADPHONE_INSERT, ABS_Z, ABS_RZ`.
- Em `event_line`, trocar o braço `EV_SW if ev.code == SW_LID => "Lid".to_string(),` por:
```rust
        EV_SW if ev.code == SW_HEADPHONE_INSERT => "headphones".to_string(),
```
- Acrescentar a seguir ao braço `hat0y`:
```rust
        EV_ABS if ev.code == ABS_Z => "l2".to_string(),
        EV_ABS if ev.code == ABS_RZ => "r2".to_string(),
```

- [ ] **Step 6: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

Se algum teste de `crates/slot/tests/input.rs` falhar por enviar `EV_SW SW_LID` à espera de `Btn::Lid` vindo do hardware, mudá-lo para a forma de "tampa" que o modo PC já usa (`RawEvent::Down(Btn::Lid)` direto na fila de input). A ação interna `Lid` mantém-se.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Map the TrimUI pad and read L2 and R2 off their trigger axes

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Nós de input sem duplicados e auscultadores pelo input

**Files:**
- Modify: `crates/slot/src/input/evdev.rs` (`pick_devices`, `Picked`; sai `wanted_node` e `SW_LID`), `crates/slot/src/input/device.rs`, `crates/slot/src/input/trace.rs` (`survey`), `crates/slot/src/device_app.rs`, `crates/slot/tests/evdev.rs`

**Interfaces:**
- Consumes: `evdev::WANTED`, `Sticks`, `abs_max`, `slot_power::has_bit`
- Produces:
  - `evdev::Picked { node: PathBuf, keys: Vec<u16>, axes: bool, jack: bool }` com `wants(&self, Ev) -> bool`
  - `evdev::pick_devices(&Path, &Path) -> Vec<Picked>`
  - `DeviceInput::open(&Path, Arc<AtomicBool>)`; `DeviceInput::open_in(&Path, &Path, &Path, bool)` mantém a assinatura
  - `device::switch_on(&File, u16) -> Option<bool>`

**Antes de começar:** em `docs/brick-hardware.md` §2, ver o nome do gamepad virtual. Se não começar por `TRIMUI Player`, mudar `PAD_PREFIX` e o nome no teste.

- [ ] **Step 1: Escrever os testes**

Em `crates/slot/tests/evdev.rs`, acrescentar os helpers e testes:
```rust
fn caps(bits: &[u16]) -> String {
    let top = bits.iter().map(|b| usize::from(*b) / 64).max().unwrap_or(0);
    let mut words = vec![0u64; top + 1];
    for b in bits {
        words[usize::from(*b) / 64] |= 1 << (b % 64);
    }
    let text: Vec<String> = words.iter().rev().map(|w| format!("{w:x}")).collect();
    text.join(" ") + "\n"
}

fn node(d: &tempfile::TempDir, ev: &str, name: &str, keys: &[u16], abs: &[u16], sw: &[u16]) {
    let sys = d.path().join("sys/class/input").join(ev).join("device");
    write(&d.path().join("dev/input").join(ev), "");
    write(&sys.join("name"), &format!("{name}\n"));
    write(&sys.join("capabilities/key"), &caps(keys));
    write(&sys.join("capabilities/abs"), &caps(abs));
    write(&sys.join("capabilities/sw"), &caps(sw));
}

fn brick_tree() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    node(&d, "event0", "axp2202-pek", &[116], &[], &[]);
    node(&d, "event1", "gpio-keys", &[0x130, 0x131, 0x133, 0x134, 114, 115], &[], &[]);
    node(&d, "event2", "headphone-jack", &[], &[], &[0x02]);
    node(
        &d,
        "event3",
        "TRIMUI Player1",
        &[0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 0x13d, 0x13e],
        &[0x00, 0x01, 0x02, 0x05, 0x10, 0x11],
        &[],
    );
    d
}

fn picked_in(d: &tempfile::TempDir) -> Vec<slot::input::evdev::Picked> {
    pick_devices(
        &d.path().join("dev/input"),
        &d.path().join("sys/class/input"),
    )
}

fn named(picked: &[slot::input::evdev::Picked], ev: &str) -> slot::input::evdev::Picked {
    picked
        .iter()
        .find(|p| p.node.ends_with(ev))
        .unwrap_or_else(|| panic!("{ev} was not picked: {picked:?}"))
        .clone()
}

#[test]
fn the_virtual_pad_claims_its_buttons_before_any_raw_node() {
    let d = brick_tree();
    let picked = picked_in(&d);
    assert!(picked[0].node.ends_with("event3"), "{picked:?}");
    let pad = named(&picked, "event3");
    assert!(pad.axes);
    assert!(pad.keys.contains(&0x131) && pad.keys.contains(&0x13c));
    assert_eq!(named(&picked, "event1").keys, vec![115, 114]);
    assert_eq!(named(&picked, "event0").keys, vec![116]);
    assert!(named(&picked, "event2").jack);
}

#[test]
fn a_raw_node_that_only_repeats_the_pads_buttons_is_not_opened() {
    let d = tempfile::tempdir().unwrap();
    node(&d, "event1", "gpio-keys", &[0x130, 0x131], &[], &[]);
    node(&d, "event3", "TRIMUI Player1", &[0x130, 0x131], &[0x10, 0x11], &[]);
    let picked = picked_in(&d);
    assert_eq!(picked.len(), 1, "{picked:?}");
    assert!(picked[0].node.ends_with("event3"));
}

#[test]
fn a_picked_node_passes_only_what_it_claimed() {
    use slot::input::evdev::{Ev, Picked, EV_ABS};
    let p = Picked {
        node: "event1".into(),
        keys: vec![115],
        axes: false,
        jack: false,
    };
    let ev = |kind, code| Ev {
        kind,
        code,
        value: 1,
    };
    assert!(p.wants(ev(EV_KEY, 115)));
    assert!(!p.wants(ev(EV_KEY, 0x130)));
    assert!(!p.wants(ev(EV_ABS, 0x10)));
    assert!(!p.wants(ev(EV_SW, 0x02)));
}

#[test]
fn a_lid_switch_alone_no_longer_opens_a_node() {
    let d = tempfile::tempdir().unwrap();
    node(&d, "event2", "gpio-keys-lid", &[], &[], &[0x00]);
    assert!(picked_in(&d).is_empty());
}
```

Atualizar os testes antigos que dependem da tampa:
- `devices_are_picked_by_capability_and_never_by_position`: trocar o `let names: Vec<_> = picked.iter().filter_map(|p| p.file_name())` por `picked.iter().filter_map(|p| p.node.file_name())`, e o `assert_eq!(names, ["event1", "event2"]);` por `assert_eq!(names, ["event1"]);`.
- `the_survey_reports_the_nodes_that_were_not_opened_too`: trocar `assert!(line("event2").contains("opened"), ...)` por `assert!(line("event2").contains("skipped"), "{}", line("event2"));`.

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot --test evdev
```

Esperado: falha de compilação (`Picked` não existe; `pick_devices` devolve `Vec<PathBuf>`).

- [ ] **Step 3: Implementar a seleção em `crates/slot/src/input/evdev.rs`**

Apagar `SW_LID`, a função `wanted_node` e a `pick_devices` antiga. Acrescentar:
```rust
const PAD_PREFIX: &str = "TRIMUI Player";
const AXES: [u16; 4] = [ABS_HAT0X, ABS_HAT0Y, ABS_Z, ABS_RZ];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picked {
    pub node: PathBuf,
    pub keys: Vec<u16>,
    pub axes: bool,
    pub jack: bool,
}

impl Picked {
    pub fn wants(&self, ev: Ev) -> bool {
        match ev.kind {
            EV_KEY => self.keys.contains(&ev.code),
            EV_ABS => self.axes,
            EV_SW => self.jack && ev.code == SW_HEADPHONE_INSERT,
            _ => false,
        }
    }
}

pub fn pick_devices(dev: &Path, sys: &Path) -> Vec<Picked> {
    let Ok(entries) = std::fs::read_dir(dev) else {
        return Vec::new();
    };
    let mut nodes: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("event"))
        })
        .collect();
    nodes.sort();
    nodes.sort_by_key(|node| !device_name(sys, node).starts_with(PAD_PREFIX));
    let (mut taken, mut axes_taken, mut jack_taken) = (Vec::new(), false, false);
    let mut picked = Vec::new();
    for node in nodes {
        let Some(name) = node.file_name() else {
            continue;
        };
        let device = sys.join(name).join("device");
        let cap = |file: &str| {
            std::fs::read_to_string(device.join("capabilities").join(file)).unwrap_or_default()
        };
        let (key, abs, sw) = (cap("key"), cap("abs"), cap("sw"));
        let keys: Vec<u16> = WANTED
            .iter()
            .copied()
            .filter(|k| has_bit(&key, *k) && !taken.contains(k))
            .collect();
        let axes = !axes_taken && AXES.iter().any(|a| has_bit(&abs, *a));
        let jack = !jack_taken && has_bit(&sw, SW_HEADPHONE_INSERT);
        if keys.is_empty() && !axes && !jack {
            continue;
        }
        taken.extend(&keys);
        axes_taken |= axes;
        jack_taken |= jack;
        picked.push(Picked {
            node,
            keys,
            axes,
            jack,
        });
    }
    picked
}
```

- [ ] **Step 4: Reescrever a leitura em `crates/slot/src/input/device.rs`**

1. A struct fica só com a fila:
```rust
pub struct DeviceInput {
    pending: Arc<Mutex<Vec<RawEvent>>>,
}
```
2. Substituir `open`, `open_in` e `open_traced` por:
```rust
impl DeviceInput {
    pub fn open(root: &Path, jack: Arc<AtomicBool>) -> Self {
        DeviceInput::open_with(Path::new(DEV), Path::new(SYS), root, trace::enabled(), jack)
    }

    pub fn open_in(dev: &Path, sys: &Path, root: &Path, trace: bool) -> Self {
        DeviceInput::open_with(dev, sys, root, trace, Arc::default())
    }

    fn open_with(dev: &Path, sys: &Path, root: &Path, trace: bool, jack: Arc<AtomicBool>) -> Self {
        let trace = trace.then(|| Trace::start(root, dev, sys)).flatten();
        let pending: Arc<Mutex<Vec<RawEvent>>> = Arc::new(Mutex::new(Vec::new()));
        for picked in pick_devices(dev, sys) {
            eprintln!(
                "slot: input {} ({}) keys {:x?} axes {} jack {}",
                picked.node.display(),
                device_name(sys, &picked.node),
                picked.keys,
                picked.axes,
                picked.jack
            );
            quicken_poll(sys, &picked.node);
            let queue = pending.clone();
            let name = picked.node.display().to_string();
            let trace = trace.clone();
            let jack = jack.clone();
            let spawned = std::thread::Builder::new()
                .name(format!("slot-input-{name}"))
                .spawn(move || read_node(&picked, &queue, trace.as_deref(), &jack));
            if let Err(e) = spawned {
                eprintln!("slot: input {name}: {e}");
            }
        }
        DeviceInput { pending }
    }
}
```
3. Acrescentar:
```rust
const EVIOCGSW_8: libc::c_ulong = 0x8008_451b;

pub fn switch_on(file: &File, code: u16) -> Option<bool> {
    let mut bits = [0u8; 8];
    let rc = unsafe { libc::ioctl(file.as_raw_fd(), EVIOCGSW_8 as _, bits.as_mut_ptr()) };
    (rc >= 0).then(|| bits[usize::from(code / 8)] >> (code % 8) & 1 == 1)
}
```
4. Em `read_node`, mudar a assinatura para:
```rust
fn read_node(picked: &Picked, queue: &Mutex<Vec<RawEvent>>, trace: Option<&Trace>, jack: &AtomicBool) {
```
Usar `let node = &picked.node;` no início. A seguir ao `let mut sticks = ...`, acrescentar:
```rust
    if picked.jack {
        if let Some(on) = switch_on(&file, SW_HEADPHONE_INSERT) {
            jack.store(on, Ordering::Relaxed);
        }
    }
```
5. No ciclo, depois do bloco `if let Some(trace) = trace { trace.event(&label, ev); }`, acrescentar `if !picked.wants(ev) { continue; }` e mudar o `match ev.kind` para:
```rust
            match ev.kind {
                EV_ABS => events.extend(sticks.feed(ev)),
                EV_SW => jack.store(ev.value != 0, Ordering::Relaxed),
                _ => events.extend(to_raw(ev)),
            }
```
6. Apagar `PSY`, `LID_POLL_MS`, `read_lid`, `find_hall` e os campos `hall`, `lid_shut` e `next_lid_poll`. `poll` fica:
```rust
impl InputSource for DeviceInput {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        std::mem::take(&mut *self.pending.lock().unwrap_or_else(|e| e.into_inner()))
    }
}
```
7. Ajustar os `use`:
- `use std::sync::atomic::{AtomicBool, Ordering};`
- `use super::evdev::{decode, device_name, pick_devices, to_raw, Ev, Picked, Sticks, ABS_RZ, ABS_Z, EVENT_BYTES, EV_ABS, EV_KEY, EV_SW, EV_SYN, SW_HEADPHONE_INSERT};`
- tirar `Btn` e `PathBuf` se deixarem de ser usados.

- [ ] **Step 5: `survey` em `crates/slot/src/input/trace.rs`**

Trocar `let opened = pick_devices(dev, sys);` por:
```rust
    let opened: Vec<PathBuf> = pick_devices(dev, sys).into_iter().map(|p| p.node).collect();
```
No `format!` final de cada linha, acrescentar o `abs`:
```rust
            format!(
                "{file} {state} name={:?} key=[{}] abs=[{}] sw=[{}]",
                device_name(sys, node),
                cap("key"),
                cap("abs"),
                cap("sw")
            )
```

- [ ] **Step 6: Ligar o jack em `crates/slot/src/device_app.rs`**

Acrescentar `use std::sync::atomic::AtomicBool;` e `use std::sync::Arc;`.

Antes de `let platform = DevicePlatform::new(root.clone());`, acrescentar:
```rust
    let jack = Arc::new(AtomicBool::new(false));
```
Trocar `let mut input = DeviceInput::open(&root);` por:
```rust
    let mut input = DeviceInput::open(&root, jack);
```
Na Tarefa 7, a plataforma, que é criada antes, passa a receber `jack.clone()`.

- [ ] **Step 7: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Open each input node for the codes no earlier node claimed, and read the jack

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Plataforma Brick: brilho, rumble por GPIO e auscultadores

**Files:**
- Modify (reescrever): `crates/slot-power/src/device.rs`
- Modify: `crates/slot-power/src/lib.rs`, `crates/slot-power/tests/device.rs`, `crates/slot/src/device_app.rs`

**Interfaces:**
- Consumes: o trait `Platform` (Tarefa 3)
- Produces:
  - `DevicePlatform::new(PathBuf, Arc<AtomicBool>)`
  - `DevicePlatform::probe(sysfs: &Path, disp: &Path, root: PathBuf, jack: Arc<AtomicBool>)`
  - `slot_power::{backlight_value(u8) -> u32, BACKLIGHT: [u32; 10], motor_voltage(u16) -> u32, MOTOR_MIN_UV, MOTOR_MAX_UV, RUMBLE_GPIO}`
  - mantêm-se `has_bit`, `motor_change`, `read_only`, `read_only_in`, `record_first_frame`, `trace_first_frame`, `uptime_seconds`
  - sai `rumble_node`

- [ ] **Step 1: Reescrever as fixtures e os testes**

Em `crates/slot-power/tests/device.rs`:

**Substituir o topo** (os `use` até ao fim de `fn platform`) por:
```rust
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use slot_power::{
    backlight_value, motor_change, motor_voltage, Battery, Charge, DevicePlatform, LedState,
    Platform, BACKLIGHT, MOTOR_MAX_UV, MOTOR_MIN_UV,
};
use tempfile::TempDir;

fn sysfs_with(capacity: &str, status: Option<&str>) -> TempDir {
    let d = tempfile::tempdir().unwrap();
    let supply = d.path().join("class/power_supply");
    fs::create_dir_all(supply.join("axp2202-usb")).unwrap();
    fs::write(supply.join("axp2202-usb/type"), "USB\n").unwrap();
    fs::create_dir_all(supply.join("axp2202-battery")).unwrap();
    fs::write(supply.join("axp2202-battery/type"), "Battery\n").unwrap();
    fs::write(
        supply.join("axp2202-battery/capacity"),
        format!("{capacity}\n"),
    )
    .unwrap();
    if let Some(s) = status {
        fs::write(supply.join("axp2202-battery/status"), format!("{s}\n")).unwrap();
    }
    let gpio = d.path().join("class/gpio/gpio227");
    fs::create_dir_all(&gpio).unwrap();
    fs::write(gpio.join("value"), "0\n").unwrap();
    fs::write(gpio.join("direction"), "in\n").unwrap();
    fs::create_dir_all(d.path().join("class/motor")).unwrap();
    fs::write(d.path().join("class/motor/voltage"), "3300000\n").unwrap();
    d
}

fn sysfs(capacity: &str) -> TempDir {
    sysfs_with(capacity, Some("Discharging"))
}

fn charge_of(status: Option<&str>) -> Charge {
    platform(&sysfs_with("87", status)).charge()
}

fn read(d: &TempDir, path: &str) -> String {
    fs::read_to_string(d.path().join(path))
        .unwrap()
        .trim()
        .to_string()
}

fn platform(d: &TempDir) -> DevicePlatform {
    platform_jacked(d, Arc::default())
}

fn platform_jacked(d: &TempDir, jack: Arc<AtomicBool>) -> DevicePlatform {
    DevicePlatform::probe(
        d.path(),
        &d.path().join("dev-disp"),
        PathBuf::from("/mnt/SDCARD/Slot"),
        jack,
    )
}
```

**Apagar** os testes:
- `the_backlight_steps_span_whatever_range_the_kernel_reports`
- `step_zero_is_dark`
- `dispdbg_tree`, `dispdbg`, `a_tree_with_no_backlight_class_drives_the_panel_through_dispdbg`
- `a_backlight_class_still_wins_over_dispdbg`
- `the_motor_is_found_by_capability_rather_than_by_position`
- `a_tree_with_no_force_feedback_has_no_motor`
- o helper `leds` e o `read` antigo
- `a_multicolour_led_gets_a_colour_per_state`, `a_mono_led_is_on_except_when_the_battery_is_low`, `a_tree_with_no_led_is_a_silent_no_op`
- `with_jack`, `headphones_follow_the_detection_pin`

**Nos testes que ficam**, trocar as chamadas `sysfs("255", X)` por `sysfs(X)` e `sysfs_with("255", X, S)` por `sysfs_with(X, S)`:
```sh
sed -i 's/sysfs("255", /sysfs(/g; s/sysfs_with("255", /sysfs_with(/g' crates/slot-power/tests/device.rs
```

**Substituir** `the_probe_reports_what_it_found`, `a_probe_that_found_nothing_says_so_rather_than_saying_nothing` e `the_report_says_whether_there_is_an_rtc` por:
```rust
#[test]
fn the_probe_reports_what_it_found() {
    let r = platform(&sysfs("87")).report();
    assert!(r.contains("axp2202-battery"), "{r}");
    assert!(r.contains("motor gpio227"), "{r}");
}

#[test]
fn a_probe_that_found_nothing_says_so_rather_than_saying_nothing() {
    let d = tempfile::tempdir().unwrap();
    let r = platform(&d).report();
    assert!(r.contains("no battery") && r.contains("no motor"), "{r}");
}

#[test]
fn the_report_says_whether_there_is_an_rtc() {
    let d = sysfs("87");
    assert!(platform(&d).report().contains("no rtc"));
    fs::create_dir_all(d.path().join("class/rtc/rtc0")).unwrap();
    assert!(platform(&d).report().contains("rtc0"));
}
```

**Acrescentar:**
```rust
#[test]
fn the_backlight_follows_the_bricks_own_curve() {
    assert_eq!(backlight_value(0), 0, "step zero is dark, for doze");
    assert_eq!(backlight_value(1), 8);
    assert_eq!(backlight_value(9), 255);
    assert_eq!(backlight_value(200), 255, "a step past the top is the top");
    assert!(BACKLIGHT.windows(2).all(|w| w[0] < w[1]), "the curve only rises");
}

#[test]
fn a_backlight_with_no_display_device_is_a_silent_no_op() {
    let mut p = platform(&sysfs("87"));
    p.set_backlight(5);
}

#[test]
fn the_motor_runs_through_the_gpio_and_its_voltage() {
    let d = sysfs("87");
    let mut p = platform(&d);
    assert_eq!(read(&d, "class/gpio/gpio227/direction"), "out");
    p.set_rumble(u16::MAX / 2);
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "1");
    let uv: u32 = read(&d, "class/motor/voltage").parse().unwrap();
    assert!((1_850_000..=1_950_000).contains(&uv), "half strength wrote {uv}");
    p.set_rumble(0);
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "0");
}

#[test]
fn the_motor_voltage_spans_the_drivers_range() {
    assert_eq!(motor_voltage(u16::MAX), MOTOR_MAX_UV);
    assert_eq!(motor_voltage(0), MOTOR_MAX_UV);
    assert!(motor_voltage(1) < MOTOR_MIN_UV + 100);
}

#[test]
fn a_motor_gpio_not_yet_exported_is_exported() {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join("class/gpio")).unwrap();
    fs::write(d.path().join("class/gpio/export"), "").unwrap();
    let p = platform(&d);
    assert_eq!(read(&d, "class/gpio/export"), "227");
    assert!(p.report().contains("no motor"), "the gpio never appeared");
}

#[test]
fn headphones_follow_the_jack_switch_the_input_reads() {
    let d = sysfs("87");
    let jack = Arc::new(AtomicBool::new(false));
    let p = platform_jacked(&d, jack.clone());
    assert!(!p.headphones());
    jack.store(true, Ordering::Relaxed);
    assert!(p.headphones());
}
```

Os `use` de `Battery` e `LedState` podem ficar sem uso até à Tarefa 8. Se o clippy se queixar, tirá-los agora e voltar a pô-los na Tarefa 8.

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot-power --test device
```

Esperado: falha de compilação (`backlight_value`, `motor_voltage` e a nova assinatura de `probe` não existem).

- [ ] **Step 3: Reescrever `crates/slot-power/src/device.rs`**

Manter **exatamente**:
- `uptime_seconds`, `record_first_frame`, `trace_first_frame`, `RUN_DIR`
- `has_bit`, `motor_change`
- `first_dir`, `is_battery`, `read_number`, `charge_at`
- `read_only`, `read_only_in`
- os métodos `breadcrumb` e `trace_boot`, e `ran`
- no `impl Platform`: `battery`, `charge`, `card_read_only`, `restart`, `poweroff`, `root`, `now`, `set_clock`

Apagar todo o resto e escrever:
```rust
use std::ffi::{c_int, c_ulong, c_void};
use std::fs;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{Battery, Charge, LedState, Platform};

pub const BACKLIGHT: [u32; 10] = [0, 8, 16, 32, 48, 72, 96, 128, 176, 255];

const DISP_LCD_SET_BRIGHTNESS: c_ulong = 0x102;
const DISP_LCD_GET_BRIGHTNESS: c_ulong = 0x103;

pub const RUMBLE_GPIO: u32 = 227;
pub const MOTOR_MIN_UV: u32 = 500_000;
pub const MOTOR_MAX_UV: u32 = 3_300_000;

pub fn backlight_value(step: u8) -> u32 {
    BACKLIGHT[usize::from(step).min(BACKLIGHT.len() - 1)]
}

pub fn motor_voltage(strength: u16) -> u32 {
    match strength {
        0 | u16::MAX => MOTOR_MAX_UV,
        s => {
            let span = u64::from(MOTOR_MAX_UV - MOTOR_MIN_UV);
            MOTOR_MIN_UV + (u64::from(s) * span / u64::from(u16::MAX)) as u32
        }
    }
}

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn disp(dev: &Path, request: c_ulong, value: u32) -> Option<i32> {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dev)
        .ok()?;
    let mut args: [c_ulong; 4] = [0, c_ulong::from(value), 0, 0];
    let rc = unsafe { ioctl(file.as_raw_fd(), request, args.as_mut_ptr() as *mut c_void) };
    (rc >= 0).then_some(rc)
}

struct GpioMotor {
    value: PathBuf,
    voltage: Option<PathBuf>,
    running: bool,
}

impl GpioMotor {
    fn open(sysfs: &Path) -> Option<GpioMotor> {
        let gpio = sysfs.join("class/gpio");
        let dir = gpio.join(format!("gpio{RUMBLE_GPIO}"));
        if !dir.is_dir() {
            let _ = fs::write(gpio.join("export"), RUMBLE_GPIO.to_string());
        }
        let value = dir.join("value");
        if !value.is_file() {
            return None;
        }
        let _ = fs::write(dir.join("direction"), "out");
        let voltage = Some(sysfs.join("class/motor/voltage")).filter(|p| p.is_file());
        Some(GpioMotor {
            value,
            voltage,
            running: false,
        })
    }

    fn play(&mut self, strength: u16) {
        let on = strength > 0;
        if on {
            if let Some(v) = &self.voltage {
                let _ = fs::write(v, motor_voltage(strength).to_string());
            }
        }
        match fs::write(&self.value, if on { "1" } else { "0" }) {
            Ok(()) => self.running = on,
            Err(e) => eprintln!("slot: rumble: {e}"),
        }
    }
}

pub struct DevicePlatform {
    root: PathBuf,
    disp: PathBuf,
    battery: Option<PathBuf>,
    rtc: Option<PathBuf>,
    motor: Option<GpioMotor>,
    jack: Arc<AtomicBool>,
    brightness_before: Option<u32>,
}

impl DevicePlatform {
    pub fn new(root: PathBuf, jack: Arc<AtomicBool>) -> Self {
        let mut p = DevicePlatform::probe(Path::new("/sys"), Path::new("/dev/disp"), root, jack);
        p.brightness_before = disp(&p.disp, DISP_LCD_GET_BRIGHTNESS, 0)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0);
        p
    }

    pub fn probe(sysfs: &Path, disp: &Path, root: PathBuf, jack: Arc<AtomicBool>) -> Self {
        DevicePlatform {
            root,
            disp: disp.to_path_buf(),
            battery: first_dir(&sysfs.join("class/power_supply"), is_battery),
            rtc: first_dir(&sysfs.join("class/rtc"), |_| true),
            motor: GpioMotor::open(sysfs),
            jack,
            brightness_before: None,
        }
    }

    pub fn report(&self) -> String {
        let leaf = |p: &Option<PathBuf>, absent: &str| match p {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => absent.to_string(),
        };
        let motor = match &self.motor {
            Some(_) => format!("motor gpio{RUMBLE_GPIO}"),
            None => "no motor".to_string(),
        };
        format!(
            "backlight {}, battery {}, {}, {motor}",
            self.disp.display(),
            leaf(&self.battery, "no battery"),
            leaf(&self.rtc, "no rtc"),
        )
    }
}
```
Colar a seguir, sem mudanças, os métodos `breadcrumb` e `trace_boot` dentro do `impl DevicePlatform`, e as funções mantidas.

O `impl Platform` fica:
```rust
impl Platform for DevicePlatform {
    fn headphones(&self) -> bool {
        self.jack.load(Ordering::Relaxed)
    }

    fn card_read_only(&self) -> bool {
        read_only(&self.root)
    }

    fn set_backlight(&mut self, step: u8) {
        if disp(&self.disp, DISP_LCD_SET_BRIGHTNESS, backlight_value(step)).is_none() {
            eprintln!("slot: backlight: {} refused step {step}", self.disp.display());
        }
    }

    fn set_led(&mut self, _state: LedState) {}

    fn set_rumble(&mut self, strength: u16) {
        let Some(motor) = &mut self.motor else {
            return;
        };
        if motor_change(strength, motor.running).is_some() {
            motor.play(strength);
        }
    }

    // battery, charge, restart, poweroff, root, now, set_clock: as they were
}
```
(No código real, colar os sete métodos que ficam, em vez do comentário.)

O `set_backlight` com erro escreve no log em cada chamada. Isto só acontece se `/dev/disp` não existir, o que no Brick não é o caso.

- [ ] **Step 4: Exportações em `crates/slot-power/src/lib.rs`**

Substituir o `pub use device::{...}` por:
```rust
pub use device::{
    backlight_value, has_bit, motor_change, motor_voltage, read_only, read_only_in,
    record_first_frame, trace_first_frame, uptime_seconds, DevicePlatform, BACKLIGHT,
    MOTOR_MAX_UV, MOTOR_MIN_UV, RUMBLE_GPIO,
};
```

- [ ] **Step 5: Ligar em `crates/slot/src/device_app.rs`**

Trocar `let platform = DevicePlatform::new(root.clone());` por:
```rust
    let platform = DevicePlatform::new(root.clone(), jack.clone());
```

- [ ] **Step 6: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

Se algum teste de `crates/slot-power/tests/` (por exemplo `first_frame.rs`) importar `rumble_node`, apagar esse uso: o rumble por force-feedback já não existe.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Drive the Brick's backlight through /dev/disp and its motor through gpio227

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: LEDs F1/F2 e `release()`

**Files:**
- Modify: `crates/slot-power/src/device.rs`, `crates/slot-power/src/lib.rs`, `crates/slot-power/src/power.rs`, `crates/slot-power/tests/device.rs`
- Create: `crates/slot-power/tests/release.rs`

**Interfaces:**
- Consumes: `DevicePlatform` (Tarefa 7)
- Produces:
  - `Platform::release(&mut self)`, com implementação por omissão vazia
  - `Power::release(&mut self)`
  - `slot_power::led_hex(LedState) -> &'static str`
  - `slot_power::{LED_STATIC, LED_SCALE_F1F2}`

**Antes de começar:** em `docs/brick-hardware.md` §6, confirmar duas coisas:
- o teste ao vivo com `effect_f1=4` deu cor fixa. Se não deu, usar o efeito que a sonda mostrou e mudar `LED_STATIC`;
- o formato lido em `effect_rgb_hex_*`. Se vier com `0x`, o restauro escreve-o tal como foi lido, e é o próprio kernel que o aceita.

- [ ] **Step 1: Escrever os testes**

Acrescentar a `crates/slot-power/tests/device.rs`:
```rust
fn led_tree(d: &TempDir) {
    let dir = d.path().join("class/led_anim");
    fs::create_dir_all(&dir).unwrap();
    for (attr, value) in [
        ("max_scale_f1f2", "100"),
        ("effect_f1", "1"),
        ("effect_f2", "1"),
        ("effect_m", "2"),
        ("effect_lr", "3"),
        ("effect_rgb_hex_f1", "FFFFFF"),
        ("effect_rgb_hex_f2", "FFFFFF"),
        ("effect_rgb_hex_m", "00FFFF"),
        ("effect_rgb_hex_lr", "FF00FF"),
        ("effect_cycles_f1", "-1"),
        ("effect_cycles_f2", "-1"),
        ("effect_cycles_m", "-1"),
        ("effect_cycles_lr", "-1"),
    ] {
        fs::write(dir.join(attr), format!("{value}\n")).unwrap();
    }
}

#[test]
fn only_the_f1_and_f2_buttons_light_up() {
    let d = sysfs("87");
    led_tree(&d);
    let mut p = platform(&d);
    assert_eq!(read(&d, "class/led_anim/effect_rgb_hex_m"), "000000");
    assert_eq!(read(&d, "class/led_anim/effect_rgb_hex_lr"), "000000");
    assert_eq!(read(&d, "class/led_anim/effect_m"), "4");
    assert_eq!(read(&d, "class/led_anim/effect_lr"), "4");
    assert_eq!(read(&d, "class/led_anim/max_scale_f1f2"), "60");
    for (state, hex) in [
        (LedState::Running, "00FF00"),
        (LedState::Charging, "FF8C00"),
        (LedState::Low, "FF0000"),
        (LedState::Charged, "00FF00"),
        (LedState::Off, "000000"),
    ] {
        p.set_led(state);
        for zone in ["f1", "f2"] {
            assert_eq!(
                read(&d, &format!("class/led_anim/effect_rgb_hex_{zone}")),
                hex,
                "{state:?} on {zone}"
            );
            assert_eq!(read(&d, &format!("class/led_anim/effect_{zone}")), "4");
        }
        assert_eq!(
            read(&d, "class/led_anim/effect_rgb_hex_m"),
            "000000",
            "{state:?} lit the top bar"
        );
    }
}

#[test]
fn release_puts_the_leds_back_the_way_they_were() {
    let d = sysfs("87");
    led_tree(&d);
    let mut p = platform(&d);
    p.set_led(LedState::Low);
    p.release();
    for (attr, value) in [
        ("max_scale_f1f2", "100"),
        ("effect_f1", "1"),
        ("effect_m", "2"),
        ("effect_lr", "3"),
        ("effect_rgb_hex_f1", "FFFFFF"),
        ("effect_rgb_hex_m", "00FFFF"),
        ("effect_rgb_hex_lr", "FF00FF"),
    ] {
        assert_eq!(read(&d, &format!("class/led_anim/{attr}")), value, "{attr}");
    }
}

#[test]
fn release_stops_a_running_motor() {
    let d = sysfs("87");
    let mut p = platform(&d);
    p.set_rumble(u16::MAX);
    p.release();
    assert_eq!(read(&d, "class/gpio/gpio227/value"), "0");
}

#[test]
fn a_tree_with_no_led_anim_is_a_silent_no_op() {
    let mut p = platform(&sysfs("87"));
    p.set_led(LedState::Running);
    p.release();
}
```

`crates/slot-power/tests/release.rs`:
```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use slot_power::{Battery, Charge, LedState, Platform, Power};

struct Counting(Arc<AtomicUsize>, PathBuf);

impl Platform for Counting {
    fn set_backlight(&mut self, _step: u8) {}
    fn battery(&self) -> Option<Battery> {
        None
    }
    fn charge(&self) -> Charge {
        Charge::Unknown
    }
    fn set_led(&mut self, _state: LedState) {}
    fn poweroff(&mut self) -> ! {
        unreachable!()
    }
    fn restart(&mut self) -> ! {
        unreachable!()
    }
    fn root(&self) -> &Path {
        &self.1
    }
    fn now(&self) -> i64 {
        0
    }
    fn set_clock(&mut self, _secs: i64) {}
    fn set_rumble(&mut self, _strength: u16) {}
    fn release(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn release_reaches_the_platform() {
    let n = Arc::new(AtomicUsize::new(0));
    let mut p = Power::new(
        Box::new(Counting(n.clone(), PathBuf::new())),
        Duration::from_secs(1),
    );
    p.release();
    assert_eq!(n.load(Ordering::Relaxed), 1);
}
```

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot-power --test device --test release
```

Esperado: falha de compilação (`release` não existe).

- [ ] **Step 3: `release` no trait e no `Power`**

`crates/slot-power/src/lib.rs`, no trait `Platform`, acrescentar:
```rust
    fn release(&mut self) {}
```
Exportar também `led_hex, LED_SCALE_F1F2, LED_STATIC` no `pub use device::{...}`.

`crates/slot-power/src/power.rs`, no `impl Power`, acrescentar:
```rust
    pub fn release(&mut self) {
        self.platform.release();
    }
```

- [ ] **Step 4: LEDs em `crates/slot-power/src/device.rs`**

Acrescentar:
```rust
pub const LED_STATIC: u32 = 4;
pub const LED_SCALE_F1F2: u32 = 60;
const LED_LIT: [&str; 2] = ["f1", "f2"];
const LED_DARK: [&str; 2] = ["m", "lr"];

pub fn led_hex(state: LedState) -> &'static str {
    match state {
        LedState::Off => "000000",
        LedState::Running | LedState::Charged => "00FF00",
        LedState::Low => "FF0000",
        LedState::Charging => "FF8C00",
    }
}

fn led_attrs() -> Vec<String> {
    let mut out = vec!["max_scale_f1f2".to_string()];
    for zone in ["f1", "f2", "m", "lr"] {
        for attr in ["effect_rgb_hex", "effect_cycles", "effect"] {
            out.push(format!("{attr}_{zone}"));
        }
    }
    out
}

struct Leds {
    dir: PathBuf,
    saved: Vec<(PathBuf, String)>,
}

impl Leds {
    fn take(sysfs: &Path) -> Option<Leds> {
        let dir = sysfs.join("class/led_anim");
        if !dir.is_dir() {
            return None;
        }
        let saved = led_attrs()
            .into_iter()
            .map(|a| dir.join(a))
            .filter_map(|p| {
                let v = fs::read_to_string(&p).ok()?;
                Some((p, v.trim().to_string()))
            })
            .collect();
        let leds = Leds { dir, saved };
        for zone in LED_DARK {
            leds.colour(zone, "000000");
        }
        leds.write("max_scale_f1f2", &LED_SCALE_F1F2.to_string());
        Some(leds)
    }

    fn write(&self, attr: &str, value: &str) {
        let _ = fs::write(self.dir.join(attr), format!("{value}\n"));
    }

    fn colour(&self, zone: &str, hex: &str) {
        self.write(&format!("effect_rgb_hex_{zone}"), hex);
        self.write(&format!("effect_cycles_{zone}"), "-1");
        self.write(&format!("effect_{zone}"), &LED_STATIC.to_string());
    }

    fn show(&self, state: LedState) {
        for zone in LED_LIT {
            self.colour(zone, led_hex(state));
        }
    }

    fn restore(&self) {
        for (path, value) in &self.saved {
            let _ = fs::write(path, format!("{value}\n"));
        }
    }
}
```

Na struct `DevicePlatform`, acrescentar o campo `leds: Option<Leds>,` e, em `probe`, `leds: Leds::take(sysfs),`. No `report`, acrescentar `leds {}` com `led_anim` ou `no leds`:
```rust
        let leds = if self.leds.is_some() { "leds led_anim" } else { "no leds" };
```
e juntá-lo ao `format!` (`"..., {motor}, {leds}"`).

No `impl Platform`, substituir o `set_led` vazio e acrescentar `release`:
```rust
    fn set_led(&mut self, state: LedState) {
        if let Some(leds) = &self.leds {
            leds.show(state);
        }
    }

    fn release(&mut self) {
        if let Some(motor) = &mut self.motor {
            if motor.running {
                motor.play(0);
            }
        }
        if let Some(leds) = &self.leds {
            leds.restore();
        }
        if let Some(level) = self.brightness_before {
            let _ = disp(&self.disp, DISP_LCD_SET_BRIGHTNESS, level);
        }
    }
```

- [ ] **Step 5: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Light only the F1 and F2 LEDs, and give everything back on release

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: Volume de hardware: pôr no máximo e repor

**Files:**
- Create: `crates/slot-power/src/mixer.rs`, `crates/slot-power/tests/mixer.rs`
- Modify: `crates/slot-power/src/lib.rs`, `crates/slot-power/src/device.rs`

**Interfaces:**
- Consumes: `DevicePlatform` (Tarefas 7–8)
- Produces:
  - `slot_power::Amixer` (trait com `cget` e `cset`)
  - `slot_power::CliAmixer`
  - `slot_power::parse_values(&str) -> Option<String>`
  - `slot_power::HwVolume` com `take(&dyn Amixer, &Path) -> HwVolume` e `restore(&self, &dyn Amixer)`
  - `slot_power::LOUD: [(&str, &str); 2]`

**Antes de começar:** em `docs/brick-hardware.md` §4, confirmar os nomes `digital volume` e `DAC volume`. Ver também o valor "normal" do DAC: o NextUI usa 160 com o `digital volume` a 0. Se forem outros, mudar `LOUD`.

- [ ] **Step 1: Escrever os testes `crates/slot-power/tests/mixer.rs`**

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;

use slot_power::{parse_values, Amixer, HwVolume};

struct Fake(RefCell<HashMap<String, String>>);

impl Fake {
    fn with(controls: &[(&str, &str)]) -> Fake {
        Fake(RefCell::new(
            controls
                .iter()
                .map(|(c, v)| (c.to_string(), v.to_string()))
                .collect(),
        ))
    }

    fn get(&self, control: &str) -> Option<String> {
        self.0.borrow().get(control).cloned()
    }
}

impl Amixer for Fake {
    fn cget(&self, control: &str) -> Option<String> {
        self.get(control)
    }

    fn cset(&self, control: &str, values: &str) -> bool {
        let mut map = self.0.borrow_mut();
        match map.get_mut(control) {
            Some(v) => {
                *v = values.to_string();
                true
            }
            None => false,
        }
    }
}

#[test]
fn amixer_values_are_read_off_the_cget_line() {
    let out = "numid=5,iface=MIXER,name='DAC volume'\n  ; type=INTEGER,access=rw------,values=2,min=0,max=255,step=0\n  : values=160,160\n";
    assert_eq!(parse_values(out).as_deref(), Some("160,160"));
    assert_eq!(parse_values("amixer: Cannot find the given element"), None);
}

#[test]
fn the_hardware_is_turned_full_up_and_put_back_after() {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir_all(d.path().join("class/speaker")).unwrap();
    fs::write(d.path().join("class/speaker/mute"), "1\n").unwrap();
    let fake = Fake::with(&[("digital volume", "40"), ("DAC volume", "100,100")]);
    let v = HwVolume::take(&fake, d.path());
    assert_eq!(fake.get("digital volume").as_deref(), Some("0"));
    assert_eq!(fake.get("DAC volume").as_deref(), Some("160"));
    let mute = || fs::read_to_string(d.path().join("class/speaker/mute")).unwrap();
    assert_eq!(mute().trim(), "0");
    v.restore(&fake);
    assert_eq!(fake.get("digital volume").as_deref(), Some("40"));
    assert_eq!(fake.get("DAC volume").as_deref(), Some("100,100"));
    assert_eq!(mute().trim(), "1");
}

#[test]
fn a_control_the_card_lacks_is_left_alone() {
    let d = tempfile::tempdir().unwrap();
    let fake = Fake::with(&[("DAC volume", "160")]);
    let v = HwVolume::take(&fake, d.path());
    v.restore(&fake);
    assert_eq!(fake.get("digital volume"), None);
    assert_eq!(fake.get("DAC volume").as_deref(), Some("160"));
}
```

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot-power --test mixer
```

Esperado: falha de compilação.

- [ ] **Step 3: Implementar `crates/slot-power/src/mixer.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOUD: [(&str, &str); 2] = [("digital volume", "0"), ("DAC volume", "160")];

pub trait Amixer {
    fn cget(&self, control: &str) -> Option<String>;
    fn cset(&self, control: &str, values: &str) -> bool;
}

pub struct CliAmixer;

impl Amixer for CliAmixer {
    fn cget(&self, control: &str) -> Option<String> {
        let out = Command::new("amixer")
            .args(["cget", &format!("name={control}")])
            .output()
            .ok()?;
        parse_values(&String::from_utf8_lossy(&out.stdout))
    }

    fn cset(&self, control: &str, values: &str) -> bool {
        Command::new("amixer")
            .args(["-q", "cset", &format!("name={control}"), values])
            .status()
            .is_ok_and(|s| s.success())
    }
}

pub fn parse_values(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(": values="))
        .map(|v| v.trim().to_string())
}

pub struct HwVolume {
    saved: Vec<(String, String)>,
    speaker: Option<(PathBuf, String)>,
}

impl HwVolume {
    pub fn take(mixer: &dyn Amixer, sysfs: &Path) -> HwVolume {
        let mut saved = Vec::new();
        for (control, loud) in LOUD {
            if let Some(old) = mixer.cget(control) {
                if mixer.cset(control, loud) {
                    saved.push((control.to_string(), old));
                }
            }
        }
        let mute = sysfs.join("class/speaker/mute");
        let speaker = fs::read_to_string(&mute)
            .ok()
            .map(|old| (mute, old.trim().to_string()));
        if let Some((path, _)) = &speaker {
            let _ = fs::write(path, "0\n");
        }
        eprintln!("slot: hardware volume taken: {saved:?}");
        HwVolume { saved, speaker }
    }

    pub fn restore(&self, mixer: &dyn Amixer) {
        for (control, old) in self.saved.iter().rev() {
            mixer.cset(control, old);
        }
        if let Some((path, old)) = &self.speaker {
            let _ = fs::write(path, format!("{old}\n"));
        }
    }
}
```

Em `crates/slot-power/src/lib.rs`, acrescentar `mod mixer;` e `pub use mixer::{parse_values, Amixer, CliAmixer, HwVolume, LOUD};`.

`slot-power` precisa de `tempfile` em `[dev-dependencies]`; já o tem, porque os testes de `device` o usam.

- [ ] **Step 4: Usar no `DevicePlatform`**

Em `crates/slot-power/src/device.rs`:
- Acrescentar o campo `volume: Option<crate::mixer::HwVolume>,` e, em `probe`, `volume: None,`.
- Em `new`, antes do `p` final:
```rust
        p.volume = Some(crate::mixer::HwVolume::take(&crate::mixer::CliAmixer, Path::new("/sys")));
```
- No fim de `release`:
```rust
        if let Some(volume) = &self.volume {
            volume.restore(&crate::mixer::CliAmixer);
        }
```

O `take` corre só em `new`, que é só no dispositivo. Os testes usam `probe`, por isso nunca chamam o `amixer` real.

- [ ] **Step 5: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Turn the Brick's mixer full up for slot's own volume and restore it after

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: Pacote `Apps/Slot` e scripts do cartão

**Files:**
- Move: `card/System/coresave.sh` → `card/Apps/Slot/coresave.sh`, `card/System/slotlink.sh` → `card/Apps/Slot/slotlink.sh`, `card/System/bezels/640x480.png` → `card/Apps/Slot/bezels/640x480.png`
- Create: `card/Apps/Slot/config.json`, `card/Apps/Slot/launch.sh`, `card/Apps/Slot/icon.png`, `card/Apps/Slot/bezels/1024x768.png`, `card/Apps/Slot/boot/slot-boot.sh`, `card/test-launch.sh`
- Modify: `card/test-coresave.sh`, `card/test-slotlink.sh`, `taskfile.yml` (`test:card`)

**Interfaces:**
- Consumes: os caminhos do `Layout` (Tarefa 4): `SLOT_SD`, `SLOT_APP`
- Produces:
  - `launch.sh`, que exporta `SLOT_SD` e `SLOT_APP`, roda o `slot.log` e repõe o `core_pattern`
  - `coresave.sh` com `SLOT_DATA`
  - `boot/slot-boot.sh`

- [ ] **Step 1: Mover ficheiros**

```bash
mkdir -p card/Apps/Slot/bezels card/Apps/Slot/boot
git mv card/System/coresave.sh card/Apps/Slot/coresave.sh
git mv card/System/slotlink.sh card/Apps/Slot/slotlink.sh
git mv card/System/bezels/640x480.png card/Apps/Slot/bezels/640x480.png
```

Nos dois testes de cartão, apontar `SCRIPT` para o novo sítio:
- `card/test-coresave.sh`: `SCRIPT="$HERE/Apps/Slot/coresave.sh"`
- `card/test-slotlink.sh`: `SCRIPT="$HERE/Apps/Slot/slotlink.sh"`

- [ ] **Step 2: Escrever o teste do `launch.sh`, `card/test-launch.sh`**

```sh
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

SLOT_SD="$TMP/sd" SLOT_CORE_PATTERN="$TMP/core_pattern" sh "$TMP/app/launch.sh"

log="$TMP/sd/Slot/slot.log"
grep -q "sd=$TMP/sd app=$TMP/app pwd=$TMP/app" "$log" \
	|| { echo "slot ran without SLOT_SD and SLOT_APP: [$(cat "$log")]" >&2; exit 1; }
grep -q "pattern=|/bin/sh $TMP/app/coresave.sh %p %s %e %t" "$log" \
	|| { echo "the core pattern was not pointed at coresave.sh while slot ran" >&2; exit 1; }
[ "$(cat "$TMP/core_pattern")" = "core" ] \
	|| { echo "the core pattern was not given back" >&2; exit 1; }
[ "$(cat "$TMP/sd/Slot/slot.log.1")" = "old run" ] \
	|| { echo "the last run's log was not kept" >&2; exit 1; }

echo "launch: all passed"
```

```sh
sh card/test-launch.sh
```

Esperado: falha (`cp: cannot stat ... launch.sh`).

- [ ] **Step 3: Escrever `card/Apps/Slot/launch.sh`**

```sh
#!/bin/sh
APP="$(cd "$(dirname "$0")" && pwd)"
SD="${SLOT_SD:-/mnt/SDCARD}"
DATA="$SD/Slot"
CORE_PATTERN="${SLOT_CORE_PATTERN:-/proc/sys/kernel/core_pattern}"
mkdir -p "$DATA"
[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
OLD_CORE="$(cat "$CORE_PATTERN" 2>/dev/null)"
echo "|/bin/sh $APP/coresave.sh %p %s %e %t" > "$CORE_PATTERN" 2>/dev/null
export SLOT_SD="$SD" SLOT_APP="$APP"
cd "$APP" || exit 1
"$APP/slot" > "$DATA/slot.log" 2>&1
[ -n "$OLD_CORE" ] && echo "$OLD_CORE" > "$CORE_PATTERN" 2>/dev/null
exit 0
```

```sh
sh card/test-launch.sh
```

Esperado: `launch: all passed`.

- [ ] **Step 4: `coresave.sh` com a pasta de dados**

Em `card/Apps/Slot/coresave.sh`:
- Trocar `SD="${AGS_CARD:-/mnt/sdcard}"` por:
```sh
SD="${AGS_CARD:-/mnt/SDCARD}"
DATA="${SLOT_DATA:-$SD/Slot}"
```
- Trocar `DIR="$SD/crash"` por `DIR="$DATA/crash"`.
- Trocar `cp "$SD/slot.log"` por `cp "$DATA/slot.log"`.

Em `card/test-coresave.sh`:
- Em `run()`, acrescentar `SLOT_DATA="$TMP/card/Slot"` às variáveis.
- Em `setup()`, trocar `mkdir -p "$TMP/card"` por `mkdir -p "$TMP/card/Slot"`.
- Trocar todas as ocorrências de `$TMP/card/crash` por `$TMP/card/Slot/crash`, e `"$TMP/card/slot.log"` por `"$TMP/card/Slot/slot.log"`:
```sh
sed -i 's#\$TMP/card/crash#$TMP/card/Slot/crash#g; s#"\$TMP/card/slot.log"#"$TMP/card/Slot/slot.log"#g' card/test-coresave.sh
```

```sh
sh card/test-coresave.sh
```

Esperado: `coresave: all passed`.

- [ ] **Step 5: `config.json`, script de arranque, ícone e moldura**

`card/Apps/Slot/config.json`:
```json
{
    "label": "Slot",
    "icon": "icon.png",
    "iconsel": "icon.png",
    "launch": "launch.sh",
    "description": "Game Boy, Game Boy Color and Game Boy Advance shelf"
}
```

`card/Apps/Slot/boot/slot-boot.sh`:
```sh
#!/bin/sh
APP=/mnt/SDCARD/Apps/Slot
[ -f "$APP/launch.sh" ] || exit 0
i=0
while [ "$i" -lt 50 ] && ! grep -q "TRIMUI Player" /proc/bus/input/devices 2>/dev/null; do
	sleep 0.1
	i=$((i + 1))
done
sh "$APP/launch.sh"
```
Confirmar em `docs/brick-hardware.md` §9:
- O script de arranque stock corre `System/starts/*.sh` **em primeiro plano, antes do MainUI**? Se correr em fundo, ou depois do MainUI, **parar e perguntar ao utilizador** antes de continuar o "Start on Boot". Explicar que o mecanismo previsto não serve e qual a alternativa que o §9 sugere.
- Se o nome do gamepad na §2 não for `TRIMUI Player…`, mudar o `grep`.

Ícone e moldura (no WSL):
```sh
rsvg-convert -w 260 -h 260 --keep-aspect-ratio crates/slot-ui/assets/cart.svg -o /tmp/cart.png
convert /tmp/cart.png -gravity center -background none -extent 300x300 card/Apps/Slot/icon.png
convert card/Apps/Slot/bezels/640x480.png -filter Lanczos -resize 1024x768! card/Apps/Slot/bezels/1024x768.png
file card/Apps/Slot/icon.png card/Apps/Slot/bezels/1024x768.png
```

Esperado: `PNG image data, 300 x 300` e `PNG image data, 1024 x 768`. Mostrar o ícone ao utilizador (abrir o PNG) antes do commit.

Bits de execução no git:
```bash
git add card/Apps/Slot
git update-index --chmod=+x card/Apps/Slot/launch.sh card/Apps/Slot/coresave.sh card/Apps/Slot/slotlink.sh card/Apps/Slot/boot/slot-boot.sh
```

- [ ] **Step 6: `taskfile.yml`, tarefa `test:card`**

```yaml
  test:card:
    desc: Test the scripts slot ships on the card, off-device
    cmds:
      - sh card/test-slotlink.sh
      - sh card/test-coresave.sh
      - sh card/test-launch.sh
```

- [ ] **Step 7: Verificar**

Correr o bloco de verificação das Global Constraints, mais `sh card/test-launch.sh`. Esperado: verde. A pasta `card/System` deve estar vazia e desaparecer; se sobrar alguma coisa, `git status` mostra-a.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Package slot as a StockMix app under Apps/Slot

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: "Exit to Menu"

**Files:**
- Modify: `crates/slot-ui/src/power_menu.rs`, `crates/slot/src/frontend.rs:582-590,521-536`, `crates/slot/src/app.rs`, `crates/slot/src/device_app.rs`, `crates/slot/src/host_app.rs:71-77`, `crates/slot/tests/power.rs`

**Interfaces:**
- Consumes: `Power::release()` (Tarefa 8)
- Produces:
  - `PowerChoice::Exit`, com `ALL = [Exit, Restart, PowerOff]`
  - `App::exiting()`, `App::ready_to_exit()`, `App::exit()`
  - `Frontend::exiting()`, `Frontend::exit()`

- [ ] **Step 1: Escrever os testes**

Em `crates/slot/tests/power.rs`:
- Em `a_hold_opens_the_menu_and_commits_nothing`, trocar a mensagem `"the menu opens on Restart"` por `"the menu opens on Exit"`.
- Substituir `each_row_commits_to_its_own_outcome` por:
```rust
#[test]
fn each_row_commits_to_its_own_outcome() {
    for (down, want) in [(0, "exit"), (1, "restart"), (2, "off")] {
        let d = tmp_root_with_carts(&["Emerald"]);
        let mut a = app_playing_in(d.path(), "Emerald");
        a.apply(Action::PowerHold);
        for _ in 0..down {
            a.apply(Action::GbaDown(Btn::Down));
        }
        a.apply(Action::GbaDown(Btn::A));
        assert_eq!(
            a.power_menu(),
            None,
            "{want}: the menu closes on the choice"
        );
        match want {
            "exit" => assert!(a.exiting() && !a.restarting() && !a.powering_off()),
            "restart" => assert!(a.restarting() && !a.powering_off() && !a.exiting()),
            _ => assert!(a.powering_off() && !a.restarting() && !a.exiting()),
        }
    }
}
```
- Acrescentar:
```rust
#[test]
fn exit_shows_its_screen_first_and_leaves_the_resume_on_the_card() {
    let d = tmp_root_with_carts(&["Emerald"]);
    let mut a = app_playing_in(d.path(), "Emerald");
    assert!(matches!(PowerChoice::ALL[0], PowerChoice::Exit));
    a.apply(Action::PowerHold);
    a.apply(Action::GbaDown(Btn::A));
    assert!(a.exiting());
    assert!(!a.ready_to_exit(), "the exit screen shows before slot goes");
    assert!(a.shutting_down());
    a.exit();
    assert!(
        StateRing::new(d.path(), Platform::Gba, Core::Mgba, "Emerald")
            .read_resume()
            .unwrap()
            .is_some(),
        "the game was left without a resume point"
    );
}
```

- [ ] **Step 2: Correr e ver falhar**

```sh
cargo test -p slot --test power
```

Esperado: falha de compilação (`PowerChoice::Exit`, `exiting` não existem).

- [ ] **Step 3: `PowerChoice::Exit` em `crates/slot-ui/src/power_menu.rs`**

```rust
pub enum PowerChoice {
    Exit,
    Restart,
    PowerOff,
}

impl PowerChoice {
    pub const ALL: [PowerChoice; 3] = [PowerChoice::Exit, PowerChoice::Restart, PowerChoice::PowerOff];
```
e em `text`, acrescentar `PowerChoice::Exit => "Exit to Menu",`. Os `#[derive]` ficam como estão.

Em `crates/slot/src/frontend.rs`, no `shutdown: PowerChoice::ALL ... menu_face(match c { ... })`, acrescentar `PowerChoice::Exit => "Exiting",`.

- [ ] **Step 4: Estado de saída em `crates/slot/src/app.rs`**

1. Na struct `App`, acrescentar `exiting: bool,` junto de `restarting: bool,` e `exiting: false,` em `App::new`.
2. Em `power_menu_input`, no `match PowerChoice::ALL[index]`, acrescentar antes de `Restart`:
```rust
                    PowerChoice::Exit => {
                        self.exiting = true;
                        self.act_at = self.now() + SHUTDOWN_SHOW_MS;
                    }
```
3. Junto de `restarting()`:
```rust
    pub fn exiting(&self) -> bool {
        self.exiting
    }

    pub fn ready_to_exit(&self) -> bool {
        self.exiting && self.now() >= self.act_at
    }

    pub fn exit(&mut self) {
        self.settle_saves();
        if let Some(power) = &mut self.power {
            power.release();
        }
    }
```
4. `shutting_down` passa a `self.powering_off || self.restarting || self.exiting`.
5. No desenho do ecrã de saída, trocar o `let which = if self.restarting { ... } else { ... };` por:
```rust
            let which = if self.exiting {
                PowerChoice::Exit
            } else if self.restarting {
                PowerChoice::Restart
            } else {
                PowerChoice::PowerOff
            };
```

- [ ] **Step 5: Frontend e loops**

`crates/slot/src/frontend.rs`, junto de `restart`:
```rust
    pub fn exiting(&self) -> bool {
        self.session.app().ready_to_exit()
    }

    pub fn exit(&mut self) {
        self.session.app_mut().exit();
    }
```

`crates/slot/src/device_app.rs`, a seguir ao bloco `if frontend.powering_off() { ... }`:
```rust
        if frontend.exiting() {
            frontend.exit();
            eprintln!("slot: exit to the menu");
            return;
        }
```

`crates/slot/src/host_app.rs`, a seguir ao bloco `if self.frontend.powering_off() { ... }`:
```rust
                if self.frontend.exiting() {
                    self.frontend.exit();
                    events.exit();
                }
```

- [ ] **Step 6: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

Se outros testes do menu de energia (`tests/flush.rs`, `tests/rumble.rs`, `tests/led.rs`) carregarem em A logo ao abrir à espera de Restart, acrescentar um `Down` antes. As linhas a ver são as que fazem `PowerHold` seguido de `GbaDown(Btn::A)` sem `Down`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add Exit to Menu to the power menu, settling saves and releasing the hardware

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 12: "Start on Boot"

**Files:**
- Create: `crates/slot-store/src/boot_hook.rs`, `crates/slot-store/tests/boot_hook.rs`
- Modify: `crates/slot-store/src/lib.rs`, `crates/slot-ui/src/quick_menu.rs`, `crates/slot/src/app.rs`, `crates/slot/tests/quick_menu.rs`

**Interfaces:**
- Consumes: `Layout::{boot_hook, boot_script, for_root}` (Tarefa 4); `card/Apps/Slot/boot/slot-boot.sh` (Tarefa 10)
- Produces:
  - `slot_store::BootHook`, com `for_layout(&Layout) -> BootHook`, `installed(&self) -> bool` e `set(&self, bool) -> io::Result<()>`
  - `QuickRow::StartOnBoot`, que é o último de `QuickRow::GAME`

- [ ] **Step 1: Testes do `BootHook`, `crates/slot-store/tests/boot_hook.rs`**

```rust
use std::fs;
use std::os::unix::fs::PermissionsExt;

use slot_store::{BootHook, Layout};

#[test]
fn turning_it_on_copies_the_apps_script_into_the_boot_folder() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    fs::create_dir_all(l.boot_script().parent().unwrap()).unwrap();
    fs::write(l.boot_script(), "#!/bin/sh\necho slot\n").unwrap();
    let hook = BootHook::for_layout(&l);
    assert!(!hook.installed());
    hook.set(true).unwrap();
    assert!(hook.installed());
    assert_eq!(
        fs::read_to_string(l.boot_hook()).unwrap(),
        "#!/bin/sh\necho slot\n"
    );
    let mode = fs::metadata(l.boot_hook()).unwrap().permissions().mode();
    assert_ne!(mode & 0o111, 0, "the boot script cannot be run");
    hook.set(false).unwrap();
    assert!(!hook.installed());
    hook.set(false).unwrap();
}

#[test]
fn a_missing_script_fails_and_leaves_it_off() {
    let d = tempfile::tempdir().unwrap();
    let l = Layout::stockmix(d.path(), &d.path().join("Apps/Slot"));
    let hook = BootHook::for_layout(&l);
    assert!(hook.set(true).is_err());
    assert!(!hook.installed());
}
```

```sh
cargo test -p slot-store --test boot_hook
```

Esperado: falha de compilação.

- [ ] **Step 2: Implementar `crates/slot-store/src/boot_hook.rs`**

```rust
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::layout::Layout;

pub struct BootHook {
    script: PathBuf,
    target: PathBuf,
}

impl BootHook {
    pub fn for_layout(layout: &Layout) -> BootHook {
        BootHook {
            script: layout.boot_script(),
            target: layout.boot_hook().to_path_buf(),
        }
    }

    pub fn installed(&self) -> bool {
        self.target.is_file()
    }

    pub fn set(&self, on: bool) -> io::Result<()> {
        if !on {
            return match fs::remove_file(&self.target) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                other => other,
            };
        }
        if let Some(dir) = self.target.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::copy(&self.script, &self.target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.target, fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }
}
```

`crates/slot-store/src/lib.rs`: acrescentar `mod boot_hook;` e `pub use boot_hook::BootHook;`.

```sh
cargo test -p slot-store --test boot_hook
```

Esperado: PASS.

- [ ] **Step 3: Testes da linha no menu, em `crates/slot/tests/quick_menu.rs`**

```rust
#[test]
fn start_on_boot_installs_and_removes_the_boot_script() {
    let (d, mut a, _) = on_carousel();
    let layout = slot_store::Layout::flat(d.path());
    std::fs::create_dir_all(layout.boot_script().parent().unwrap()).unwrap();
    std::fs::write(layout.boot_script(), "#!/bin/sh\n").unwrap();
    open_at(&mut a, QuickRow::StartOnBoot);
    assert_eq!(a.quick_value(QuickRow::StartOnBoot), Some(QuickValue::Off));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::StartOnBoot), Some(QuickValue::On));
    assert!(layout.boot_hook().is_file());
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::StartOnBoot), Some(QuickValue::Off));
    assert!(!layout.boot_hook().exists());
}

#[test]
fn start_on_boot_stays_off_when_the_app_has_no_boot_script() {
    let (d, mut a, _) = on_carousel();
    open_at(&mut a, QuickRow::StartOnBoot);
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::StartOnBoot), Some(QuickValue::Off));
    assert!(!slot_store::Layout::flat(d.path()).boot_hook().exists());
}

#[test]
fn start_on_boot_is_the_last_gameplay_row() {
    assert_eq!(QuickRow::GAME.last(), Some(&QuickRow::StartOnBoot));
    assert_eq!(QuickRow::StartOnBoot.label(), "Start on Boot");
}
```

Se `on_carousel()` devolver o `TempDir` noutra posição do tuplo, ajustar o padrão: abrir o helper no topo do ficheiro e usar o `TempDir` que ele devolve.

```sh
cargo test -p slot --test quick_menu
```

Esperado: falha de compilação (`QuickRow::StartOnBoot` não existe).

- [ ] **Step 4: A linha em `crates/slot-ui/src/quick_menu.rs`**

- No fim do `enum QuickRow`, acrescentar `StartOnBoot,`.
- `pub const ALL: [QuickRow; 15]`, com `QuickRow::StartOnBoot` no fim.
- `pub const GAME: [QuickRow; 7]`, com `QuickRow::StartOnBoot` no fim.
- Em `page()`, acrescentar `| QuickRow::StartOnBoot` ao braço que devolve `&QuickRow::GAME`.
- Em `label()`, acrescentar `QuickRow::StartOnBoot => "Start on Boot",`.

- [ ] **Step 5: Comportamento em `crates/slot/src/app.rs`**

1. Campo `start_on_boot: bool,` na struct e `start_on_boot: false,` em `App::new`.
2. Em `App::boot`, depois de `app.root = Some(root.to_path_buf());`:
```rust
        app.start_on_boot =
            slot_store::BootHook::for_layout(&slot_store::Layout::for_root(root)).installed();
```
3. Em `quick_value`, acrescentar `QuickRow::StartOnBoot => Some(QuickValue::flag(self.start_on_boot)),`.
4. Em `open_quick_row`, acrescentar `| QuickRow::StartOnBoot` à lista de linhas que não abrem nada.
5. No início de `change_setting`, antes de `let s = &mut self.state;`:
```rust
        if row == QuickRow::StartOnBoot {
            return self.toggle_start_on_boot();
        }
```
e acrescentar `| QuickRow::StartOnBoot` ao braço final `QuickRow::DateTime | QuickRow::About | ... => return`.
6. Novo método:
```rust
    fn toggle_start_on_boot(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let hook = slot_store::BootHook::for_layout(&slot_store::Layout::for_root(&root));
        if let Err(e) = hook.set(!self.start_on_boot) {
            eprintln!("slot: start on boot: {e}");
        }
        self.start_on_boot = hook.installed();
    }
```

- [ ] **Step 6: Verificar**

Correr o bloco de verificação das Global Constraints. Esperado: verde.

Se o compilador apontar outro `match` exaustivo sobre `QuickRow` (por exemplo em `frontend.rs`), acrescentar `QuickRow::StartOnBoot` ao lado de `QuickRow::EjectSave`, com o mesmo tratamento.

O teste `labels_start_and_values_end_thirty_two_pixels_in` procura o primeiro valor `On` não selecionado. Se passar a apanhar o "Start on Boot", que está a Off, não deve mudar nada; se falhar, ler a mensagem e ajustar a linha que ele abre.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add Start on Boot to the Gameplay settings

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 13: Ligar o `Layout` StockMix no arranque do dispositivo

**Files:**
- Modify: `crates/slot/src/device_app.rs`

**Interfaces:**
- Consumes: `Layout::stockmix`, `Layout::install`, `app_dir` (Tarefa 4); `jack` (Tarefas 6–7)
- Produces: um binário de dispositivo que lê `SLOT_SD` (por omissão `/mnt/SDCARD`) e `SLOT_APP`, com os dados em `SD/Slot`

- [ ] **Step 1: Trocar a raiz em `device_app.rs`**

Trocar `const CARD: &str = "/mnt/sdcard";` por `const SD: &str = "/mnt/SDCARD";`.

Substituir:
```rust
    let root = std::env::var_os("SLOT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(CARD));
```
por:
```rust
    let sd = std::env::var_os("SLOT_SD")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SD));
    let layout = slot_store::Layout::stockmix(&sd, &slot_store::app_dir());
    eprintln!(
        "slot: data {}, roms {}, app {}",
        layout.data().display(),
        layout.roms(slot_store::Platform::Gba).display(),
        layout.app().display()
    );
    let root = layout.data().to_path_buf();
    layout.install();
```

- [ ] **Step 2: Verificar**

```sh
cargo clippy -p slot --no-default-features --features device -- -D warnings
```

Correr também o bloco de verificação das Global Constraints. Esperado: verde.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Boot the device build on the StockMix layout from SLOT_SD and SLOT_APP

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 14: Link por WiFi no Brick

**Files:**
- Modify: `card/Apps/Slot/slotlink.sh`, `card/test-slotlink.sh`

**Interfaces:**
- Consumes: `docs/brick-hardware.md` §10–§11 e §13
- Produces: `slotlink.sh` com
  - `RUN` em `/run` ou, se não der, em `/tmp`
  - interface AP configurável, criada com `iw` se faltar
  - reposição do WiFi stock em `link down`

**Antes de começar:** ler `docs/brick-hardware.md` §13 "Link por WiFi". **Se o chip não suportar modo AP**, não fazer esta tarefa. Em vez disso:
- avisar o utilizador;
- deixar o script como está (a falha de rádio já aparece como recusa na UI);
- seguir para a Tarefa 15.

- [ ] **Step 1: Escrever os testes novos em `card/test-slotlink.sh`**

1. Em `reset()`, acrescentar:
```sh
	make_fake iw "$TMP/iw.log" "mkdir -p \"$TMP/net/wlan1\""
	make_fake restore "$TMP/restore.log"
```
2. Em `run()`, acrescentar ao ambiente:
```sh
		AGS_IW="$TMP/bin/iw" AGS_LINK_RESTORE="$TMP/bin/restore" \
```
3. Antes do bloco final `if [ "$FAILS" -eq 0 ]`, acrescentar:
```sh
reset yes
rmdir "$TMP/net/wlan1"
run absent link host && rc=0 || rc=$?
[ "$rc" = 0 ] || fail "host without wlan1 exited $rc"
grep -q 'dev wlan0 interface add wlan1 type __ap' "$TMP/iw.log" 2>/dev/null \
	|| fail "host did not make an AP interface"

reset yes
run absent link join >/dev/null
run absent link down >/dev/null
[ -e "$TMP/restore.log" ] || fail "down after a join left the console off its own WiFi"
[ -e "$TMP/run/slotlink.wifi-taken" ] && fail "down left the wifi-taken mark"

reset yes
run absent link host >/dev/null
run absent link down >/dev/null
[ -e "$TMP/restore.log" ] && fail "down after hosting on wlan1 restarted the WiFi"
```

```sh
sh card/test-slotlink.sh
```

Esperado: `slotlink: 3 failed` (ou mais).

- [ ] **Step 2: Alterar `card/Apps/Slot/slotlink.sh`**

1. Trocar a linha `RUN="${AGS_RUN:-/run}"` por:
```sh
if [ -n "${AGS_RUN:-}" ]; then
	RUN="$AGS_RUN"
elif [ -d /run ] && [ -w /run ]; then
	RUN=/run
else
	RUN=/tmp
fi
```
2. Depois de `LINK_MARK="$RUN/slotlink.session"`, acrescentar (os valores por omissão vêm da §13; os mostrados são os previstos):
```sh
LINK_AP_IF="${AGS_LINK_AP_IF:-wlan1}"
LINK_RESTORE="${AGS_LINK_RESTORE:-/etc/wifi/wifi_init.sh start}"
LINK_STOCK_CTRL="${AGS_LINK_STOCK_CTRL:-/etc/wifi/sockets}"
LINK_TAKEN="$RUN/slotlink.wifi-taken"
IW="${AGS_IW:-iw}"
```
3. Em `link_clear`, `link_host`, `link_down` e `link_warm`, trocar cada `wlan1` literal por `"$LINK_AP_IF"`, incluindo dentro de `supplicant_stop "$RUN/wpa_ap" wlan1`.
4. Em `link_host`, trocar `wait_dev wlan1 4 || return 1` por:
```sh
	if ! wait_dev "$LINK_AP_IF" 4; then
		$IW dev wlan0 interface add "$LINK_AP_IF" type __ap >/dev/null 2>&1 || true
		wait_dev "$LINK_AP_IF" 4 || return 1
	fi
	if [ "$LINK_AP_IF" = wlan0 ]; then
		: > "$LINK_TAKEN"
		supplicant_stop "$LINK_STOCK_CTRL" wlan0
		supplicant_stop "$CTRL_DIR" wlan0
	fi
```
5. Em `link_join`, logo depois de `link_clear`:
```sh
	: > "$LINK_TAKEN"
	supplicant_stop "$LINK_STOCK_CTRL" wlan0
```
6. Em `link_down`, antes do `return 0`:
```sh
	if [ -f "$LINK_TAKEN" ]; then
		/bin/rm -f "$LINK_TAKEN"
		trace "restore: $LINK_RESTORE"
		sh -c "$LINK_RESTORE" >/dev/null 2>&1 || true
	fi
```
7. Se a §13 disser que a frequência AP tem de ser 2,4 GHz, trocar `LINK_FREQ="${AGS_LINK_FREQ:-5745}"` por `LINK_FREQ="${AGS_LINK_FREQ:-2437}"` e ajustar no teste os `grep` de `5745` para `2437`.

- [ ] **Step 3: Correr**

```sh
sh card/test-slotlink.sh
```

Esperado: `slotlink: all passed`.

- [ ] **Step 4: Verificar e commit**

Correr o bloco de verificação das Global Constraints.

```bash
git add -A
git commit -m "Make the WiFi link fit the Brick: AP interface, run dir and giving the WiFi back

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 15: Build para o dispositivo com zig, pacote e deploy

**Files:**
- Create: `tools/zig/aarch64-cc`, `tools/zig/aarch64-c++`, `tools/zig/aarch64-ar`, `tools/zig/aarch64-ranlib`, `tools/glibc-floor.sh`
- Modify: `cores/mgba/build.sh`, `cores/gpsp/build.sh`, `taskfile.yml`
- Delete: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `card/Apps/Slot` (Tarefa 10), `--init-sd` (Tarefa 4), `BRICK_GLIBC` (Tarefa 2 §1)
- Produces: as tarefas `build:device`, `core:device`, `core:gpsp`, `dist:device` (gera `dist-brick/`), `deploy:device BRICK=root@ip`, `deploy:adb`, `log:device`, `bench:device`, `sdcard:brick` e `run:brick`

- [ ] **Step 1: Wrappers do zig**

`tools/zig/aarch64-cc`:
```sh
#!/bin/sh
exec zig cc -target "aarch64-linux-gnu.${BRICK_GLIBC:-2.31}" "$@"
```
`tools/zig/aarch64-c++`:
```sh
#!/bin/sh
exec zig c++ -target "aarch64-linux-gnu.${BRICK_GLIBC:-2.31}" "$@"
```
`tools/zig/aarch64-ar`:
```sh
#!/bin/sh
exec zig ar "$@"
```
`tools/zig/aarch64-ranlib`:
```sh
#!/bin/sh
exec zig ranlib "$@"
```

`tools/glibc-floor.sh`:
```sh
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
```

```bash
git add tools/zig tools/glibc-floor.sh
git update-index --chmod=+x tools/zig/aarch64-cc tools/zig/aarch64-c++ tools/zig/aarch64-ar tools/zig/aarch64-ranlib tools/glibc-floor.sh
```

- [ ] **Step 2: Modo cross em `cores/mgba/build.sh`**

- Depois de `device_cflags="-flto=auto"`, acrescentar `cross_cflags="-O2 -mcpu=cortex-a53"`.
- Em `stamp()`, acrescentar `echo "cross_cflags=$cross_cflags"` a seguir à linha de `device_cflags`.
- Em `build()`, substituir o bloco desde `cflags=""` até ao primeiro `cmake -S ...` (inclusive) por:
```sh
	cflags=""
	cross=""
	if [ -n "${SLOT_CROSS_CC:-}" ]; then
		cflags="$cross_cflags"
		cross="-DCMAKE_SYSTEM_NAME=Linux -DCMAKE_SYSTEM_PROCESSOR=aarch64 -DCMAKE_C_COMPILER=$SLOT_CROSS_CC -DCMAKE_CXX_COMPILER=$SLOT_CROSS_CXX -DCMAKE_AR=$SLOT_CROSS_AR -DCMAKE_RANLIB=$SLOT_CROSS_RANLIB -DCMAKE_FIND_ROOT_PATH=/var/empty -DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY -DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY -DCMAKE_FIND_ROOT_PATH_MODE_PACKAGE=ONLY"
	elif [ "$(uname -s)-$(uname -m)" = "Linux-aarch64" ]; then
		cflags="$device_cflags"
	fi
	# shellcheck disable=SC2086
	cmake -S "$src" -B "$obj" $cross -DLIBMGBA_ONLY=ON -DBUILD_LIBRETRO=ON -DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_C_FLAGS="$cflags" >/dev/null
```
(Este é o único comentário novo, para o shellcheck, porque `$cross` tem de ser partido em palavras.)

- [ ] **Step 3: Modo cross em `cores/gpsp/build.sh`**

Substituir a chamada `CFLAGS="$device_cflags" make -C "$src" platform=arm64 \ ...` por:
```sh
	if [ -n "${SLOT_CROSS_CC:-}" ]; then
		CFLAGS="$device_cflags" make -C "$src" platform=arm64 \
			CC="$SLOT_CROSS_CC" CXX="$SLOT_CROSS_CXX" AR="$SLOT_CROSS_AR" \
			GIT_VERSION="\"$(printf %s "$commit" | cut -c1-7)\"" \
			-j"$(getconf _NPROCESSORS_ONLN)"
	else
		CFLAGS="$device_cflags" make -C "$src" platform=arm64 \
			GIT_VERSION="\"$(printf %s "$commit" | cut -c1-7)\"" \
			-j"$(getconf _NPROCESSORS_ONLN)"
	fi
```

- [ ] **Step 4: Tarefas do Brick no `taskfile.yml`**

1. Em `vars:`, acrescentar:
```yaml
  BRICK_GLIBC: '2.31'
  BRICK_TARGET: aarch64-unknown-linux-gnu
  BRICK_BIN: target-device/aarch64-unknown-linux-gnu/device/slot
  CROSS: >-
    BRICK_GLIBC={{.BRICK_GLIBC}}
    SLOT_CROSS_CC=$PWD/tools/zig/aarch64-cc SLOT_CROSS_CXX=$PWD/tools/zig/aarch64-c++
    SLOT_CROSS_AR=$PWD/tools/zig/aarch64-ar SLOT_CROSS_RANLIB=$PWD/tools/zig/aarch64-ranlib
```
(Usar o valor de §1/§13 de `docs/brick-hardware.md` se não for 2.31.)

2. **Apagar** as tarefas `dist`, `core:device`, `core:gpsp`, `core:gpsp:update`, `device:sh`, `build:device`, `bench:device`, `dist:device`, `deploy:device` e `log:device`.

3. **Acrescentar:**
```yaml
  build:device:
    desc: Build slot for the TrimUI Brick with cargo-zigbuild, against glibc {{.BRICK_GLIBC}}
    cmds:
      - >-
        cargo zigbuild --profile device --target-dir target-device
        --target {{.BRICK_TARGET}}.{{.BRICK_GLIBC}}
        -p slot --no-default-features --features device
      - file {{.BRICK_BIN}} | grep -q aarch64
      - sh tools/glibc-floor.sh {{.BRICK_BIN}} {{.BRICK_GLIBC}}

  core:device:
    desc: Cross-build the mGBA core for the Brick with zig
    status:
      - test -f vendor/mgba_libretro.so
      - test "$(cat vendor/mgba_libretro.so.meta 2>/dev/null)" = "$(sh cores/mgba/build.sh stamp {{.MGBA_COMMIT}})"
    cmds:
      - '{{.CROSS}} sh cores/mgba/build.sh build {{.MGBA_COMMIT}} target-device/mgba vendor/mgba_libretro.so'
      - file vendor/mgba_libretro.so | grep -q aarch64
      - sh tools/glibc-floor.sh vendor/mgba_libretro.so {{.BRICK_GLIBC}}

  core:gpsp:
    desc: >-
      Cross-build the gpSP core for the Brick from its pinned source archive, which ships beside it
      as its GPL-2.0 section 3(a) corresponding source
    status:
      - test -f vendor/gpsp_libretro.so
      - test -f vendor/gpsp-src.tar.gz
      - test "$(cat vendor/gpsp-src.sha 2>/dev/null)" = "{{.GPSP_COMMIT}}"
      - test "$(cat vendor/gpsp_libretro.so.meta 2>/dev/null)" = "$(sh cores/gpsp/build.sh stamp {{.GPSP_COMMIT}})"
    cmds:
      - |
        set -euo pipefail
        mkdir -p vendor
        rm -f vendor/gpsp_libretro.so vendor/gpsp_libretro.so.meta vendor/gpsp-src.tar.gz vendor/gpsp-src.sha vendor/gpsp-src.meta
        curl -fsSL -o vendor/gpsp-src.tar.gz "https://github.com/libretro/gpsp/archive/{{.GPSP_COMMIT}}.tar.gz"
        tar -tzf vendor/gpsp-src.tar.gz | grep "^gpsp-{{.GPSP_COMMIT}}/Makefile$" > /dev/null
      - '{{.CROSS}} sh cores/gpsp/build.sh build {{.GPSP_COMMIT}} vendor/gpsp-src.tar.gz target-device/gpsp vendor/gpsp_libretro.so'
      - file vendor/gpsp_libretro.so | grep -q aarch64
      - sh tools/glibc-floor.sh vendor/gpsp_libretro.so {{.BRICK_GLIBC}}
      - echo "{{.GPSP_COMMIT}}" > vendor/gpsp-src.sha
      - cp vendor/gpsp_libretro.so.meta vendor/gpsp-src.meta

  bench:device:
    desc: Build core_bench for the Brick. It never ships
    cmds:
      - >-
        cargo zigbuild --profile device --target-dir target-device
        --target {{.BRICK_TARGET}}.{{.BRICK_GLIBC}} -p slot-retro --example core_bench

  dist:device:
    desc: Build the Brick SD tree, OUT defaults to ./dist-brick. Copy its contents to the SD card root
    deps: [core:device, core:gpsp, build:device]
    vars:
      OUT: '{{.OUT | default "dist-brick"}}'
    cmds:
      - rm -rf {{.OUT}}
      - cargo run -q -p slot -- --init-sd {{.OUT}}
      - mkdir -p {{.OUT}}/Apps/Slot
      - cp -R card/Apps/Slot/. {{.OUT}}/Apps/Slot/
      - cp {{.BRICK_BIN}} {{.OUT}}/Apps/Slot/slot
      - cp vendor/mgba_libretro.so vendor/gpsp_libretro.so {{.OUT}}/Apps/Slot/
      - cp -r licenses {{.OUT}}/Apps/Slot/licenses
      - |
        set -euo pipefail
        sha="$(cat vendor/gpsp-src.sha)"
        cp vendor/gpsp-src.tar.gz "{{.OUT}}/Apps/Slot/licenses/gpsp-${sha}.tar.gz"
        cp vendor/gpsp-src.meta "{{.OUT}}/Apps/Slot/licenses/gpsp-${sha}.meta"
        for p in cores/gpsp/*.patch; do cp "$p" "{{.OUT}}/Apps/Slot/licenses/gpsp-$(basename "$p")"; done
        commit="$(grep '^commit=' vendor/mgba_libretro.so.meta | cut -d= -f2)"
        cp vendor/mgba_libretro.so.meta "{{.OUT}}/Apps/Slot/licenses/mgba-${commit}.meta"
        for p in cores/mgba/*.patch; do cp "$p" "{{.OUT}}/Apps/Slot/licenses/mgba-$(basename "$p")"; done
      - 'echo "Brick tree ready at {{.OUT}}: copy Apps/ and Slot/ to the root of the SD card"'

  deploy:device:
    desc: Push the Brick build over SSH. BRICK=root@<ip>; BrickTools' SSH password is tina
    requires:
      vars: [BRICK]
    deps: [dist:device]
    cmds:
      - >-
        tar -C dist-brick -cf - Apps/Slot | ssh {{.BRICK}}
        'tar -C /mnt/SDCARD -xf - && chmod +x /mnt/SDCARD/Apps/Slot/slot /mnt/SDCARD/Apps/Slot/*.sh
        /mnt/SDCARD/Apps/Slot/boot/*.sh && mkdir -p /mnt/SDCARD/Slot && sync'
      - 'echo "deployed; open Apps > Slot on the Brick, then task log:device BRICK={{.BRICK}}"'

  deploy:adb:
    desc: Push the Brick build over adb
    deps: [dist:device]
    cmds:
      - adb wait-for-device
      - adb push dist-brick/Apps/Slot /mnt/SDCARD/Apps/
      - adb shell 'chmod +x /mnt/SDCARD/Apps/Slot/slot /mnt/SDCARD/Apps/Slot/*.sh /mnt/SDCARD/Apps/Slot/boot/*.sh; mkdir -p /mnt/SDCARD/Slot; sync'

  log:device:
    desc: Read slot's log back off the Brick
    requires:
      vars: [BRICK]
    cmds:
      - ssh {{.BRICK}} 'cat /mnt/SDCARD/Slot/slot.log'

  sdcard:brick:
    desc: Make a StockMix-shaped card at ./sdcard-brick for running on this machine
    deps: [build]
    cmds:
      - ./target/debug/slot --init-sd ./sdcard-brick
      - mkdir -p sdcard-brick/Roms/GBA sdcard-brick/Roms/GB sdcard-brick/Roms/GBC sdcard-brick/BIOS
      - 'echo "put roms in sdcard-brick/Roms/<GBA|GB|GBC>, then task run:brick"'

  run:brick:
    desc: >-
      Run slot on this machine against ./sdcard-brick, laid out like a StockMix card. The host core
      tasks build macOS dylibs, so on Linux this runs the mock core (a test pattern and a tone)
    env:
      SLOT_SD: ./sdcard-brick
      SLOT_APP: ./card/Apps/Slot
      SLOT_WINDOW: '1024x768'
    cmds:
      - cargo run --release -p slot
```

4. Em `.gitignore`, acrescentar `/sdcard-brick`.

5. Na tarefa `check`, manter `lint:device` (clippy nativo da feature `device`) e `test:card`.

6. Apagar `.github/workflows/release.yml`: empacota a árvore SD2 da BaseOS e já não serve.

```bash
git rm -q .github/workflows/release.yml
```

- [ ] **Step 5: Construir tudo**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && task dist:device'
```

Esperado:
- `... glibc 2.xx <= 2.31` para `slot`, `mgba_libretro.so` e `gpsp_libretro.so`;
- `Brick tree ready at dist-brick`;
- `ls dist-brick/Apps/Slot` mostra `slot`, os dois `.so`, `launch.sh`, `config.json`, `icon.png`, `coresave.sh`, `slotlink.sh`, `boot/`, `bezels/` e `licenses/`;
- `ls dist-brick/Slot` mostra as `DATA_DIRS`.

Se o `zig cc` falhar a compilar um core, ler o erro:
- **Flag que o clang não aceita:** tirá-la de `cross_cflags`, ou do `device_cflags` do gpSP no modo cross.
- **Problema de toolchain:** passar ao plano B da spec, um container arm64 bullseye com qemu, e dizer ao utilizador antes de instalar o qemu e o podman.

- [ ] **Step 6: Verificar e commit**

Correr o bloco de verificação das Global Constraints.

```bash
git add -A
git commit -m "Cross-build slot and both cores for the Brick with zig, and package Apps/Slot

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 16: Primeiro arranque no Brick

**Files:**
- Modify: só o que os resultados obrigarem, com um teste quando a correção for lógica e não de hardware
- Possível: Tarefa 16b

**Interfaces:**
- Consumes: `task deploy:device`, `task log:device`
- Produces: o Slot a correr no Brick a partir de Apps, com imagem, botões e som; secção §14 "Primeiro arranque" em `docs/brick-hardware.md`

O utilizador tem de estar com o Brick na mão.

- [ ] **Step 1: Instalar e abrir**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && task deploy:device BRICK=root@<ip>'
```

Pedir ao utilizador para:
1. Copiar um `gba_bios.bin`, se o tiver, para uma pasta de BIOS do StockMix.
2. Abrir **Apps → Slot**.
3. Dizer o que vê.

- [ ] **Step 2: Ler o log**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && task log:device BRICK=root@<ip>'
```

Esperado, por esta ordem:
- `slot: data /mnt/SDCARD/Slot, roms /mnt/SDCARD/Roms/GBA, app /mnt/SDCARD/Apps/Slot`
- `slot: fb0 hint 1024x768`
- `slot: panel 1024x768`
- `slot: backlight /dev/disp, battery axp2202-battery, ..., motor gpio227, leds led_anim`
- `slot: hardware volume taken: [("digital volume", ...), ("DAC volume", ...)]`
- uma linha `slot: input ...` por nó: o gamepad com `axes true`, o power com `[74]` (116), o volume com `[73, 72]`
- `slot: bezel /mnt/SDCARD/Apps/Slot/bezels/1024x768.png`

- [ ] **Step 3: Árvore de decisão**

| Sintoma no log | Ação |
|---|---|
| `slot: gl context: ...` (EGL) | Fazer a **Tarefa 16b** e voltar ao Step 1 |
| Falta uma linha `slot: input` para um botão | Comparar com §2 e corrigir `WANTED`/`code_to_btn`/`PAD_PREFIX`, com teste em `tests/evdev.rs` |
| Sem som | Ver se há `slot: audio ...`. Se o `plug:default` falhar, pôr à frente em `DEVICES` (`audio/alsa.rs`) o dispositivo que `aplay -L` mostra no Brick (`ssh root@<ip> aplay -L`) |
| Ecrã preto mas log normal | Provavelmente o brilho: confirmar `/dev/disp` com `ssh root@<ip> ls -l /dev/disp` |

- [ ] **Step 4: 60 fps e desempenho dos cores**

```sh
ssh root@<ip> 'mkdir -p /mnt/SDCARD/Slot/System && touch /mnt/SDCARD/Slot/System/trace-latency.on'
```

O utilizador joga um jogo GBA durante 30 s com mGBA e 30 s com gpSP (START no cartucho troca o chip). Depois:
```sh
ssh root@<ip> 'grep "pace:" /mnt/SDCARD/Slot/slot.log | tail -6; rm /mnt/SDCARD/Slot/System/trace-latency.on'
```

Esperado: `missed a latch` perto de 0 em cada 600 frames.

Se houver muitas perdas:
- `task bench:device`
- `scp` de `target-device/aarch64-unknown-linux-gnu/device/examples/core_bench` para `/tmp`
- correr o `core_bench` no Brick
- registar o resultado em §14

- [ ] **Step 5: Registar em `docs/brick-hardware.md` §14 e commit**

```markdown
## §14 Primeiro arranque
<excerto do log; o que funcionou; correções feitas; resultados de pace>
```

```bash
git add -A
git commit -m "Record the first boot on the Brick

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 16b (só se o EGL falhar): superfície por SDL2

**Files:**
- Create: `crates/slot-gfx/src/sdl.rs`
- Modify: `crates/slot-gfx/src/lib.rs`, `crates/slot/src/device_app.rs`

**Interfaces:**
- Consumes: o trait `Surface`; `libSDL2-2.0.so.0` do sistema stock (confirmar com §3)
- Produces: `slot_gfx::SdlSurface::new() -> Result<SdlSurface, GfxError>`, que implementa `Surface`

- [ ] **Step 1: Implementar `crates/slot-gfx/src/sdl.rs`**

```rust
use std::ffi::{c_char, c_int, c_void, CStr, CString};

use libloading::Library;

use crate::surface::{GfxError, Surface};

const SDL_INIT_VIDEO: u32 = 0x20;
const SDL_WINDOW_FULLSCREEN: u32 = 0x1;
const SDL_WINDOW_OPENGL: u32 = 0x2;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_ES: c_int = 0x4;
const SDL_WINDOWPOS_UNDEFINED: c_int = 0x1FFF_0000;

#[repr(C)]
#[derive(Default)]
struct DisplayMode {
    format: u32,
    w: c_int,
    h: c_int,
    refresh_rate: c_int,
    driverdata: usize,
}

type Ptr = *mut c_void;

pub struct SdlSurface {
    lib: Library,
    window: Ptr,
    context: Ptr,
    size: (u32, u32),
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T, GfxError> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| GfxError::Context(format!("{name}: {e}")))
}

fn sdl_error(lib: &Library, what: &str) -> GfxError {
    let message = unsafe {
        sym::<unsafe extern "C" fn() -> *const c_char>(lib, "SDL_GetError")
            .map(|f| CStr::from_ptr(f()).to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    GfxError::Context(format!("{what}: {message}"))
}

impl SdlSurface {
    pub fn new() -> Result<Self, GfxError> {
        let lib = unsafe { Library::new("libSDL2-2.0.so.0") }
            .or_else(|_| unsafe { Library::new("libSDL2.so") })
            .map_err(|e| GfxError::Context(format!("libSDL2: {e}")))?;
        unsafe {
            let init: unsafe extern "C" fn(u32) -> c_int = sym(&lib, "SDL_Init")?;
            let set_attr: unsafe extern "C" fn(c_int, c_int) -> c_int =
                sym(&lib, "SDL_GL_SetAttribute")?;
            let desktop: unsafe extern "C" fn(c_int, *mut DisplayMode) -> c_int =
                sym(&lib, "SDL_GetDesktopDisplayMode")?;
            let create: unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, c_int, u32) -> Ptr =
                sym(&lib, "SDL_CreateWindow")?;
            let gl_create: unsafe extern "C" fn(Ptr) -> Ptr = sym(&lib, "SDL_GL_CreateContext")?;
            let interval: unsafe extern "C" fn(c_int) -> c_int =
                sym(&lib, "SDL_GL_SetSwapInterval")?;
            let drawable: unsafe extern "C" fn(Ptr, *mut c_int, *mut c_int) =
                sym(&lib, "SDL_GL_GetDrawableSize")?;
            if init(SDL_INIT_VIDEO) != 0 {
                return Err(sdl_error(&lib, "SDL_Init"));
            }
            set_attr(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_ES);
            set_attr(SDL_GL_CONTEXT_MAJOR_VERSION, 2);
            set_attr(SDL_GL_CONTEXT_MINOR_VERSION, 0);
            let mut mode = DisplayMode::default();
            if desktop(0, &mut mode) != 0 {
                return Err(sdl_error(&lib, "SDL_GetDesktopDisplayMode"));
            }
            let title = CString::new("slot").expect("no nul in a literal");
            let window = create(
                title.as_ptr(),
                SDL_WINDOWPOS_UNDEFINED,
                SDL_WINDOWPOS_UNDEFINED,
                mode.w,
                mode.h,
                SDL_WINDOW_OPENGL | SDL_WINDOW_FULLSCREEN,
            );
            if window.is_null() {
                return Err(sdl_error(&lib, "SDL_CreateWindow"));
            }
            let context = gl_create(window);
            if context.is_null() {
                return Err(sdl_error(&lib, "SDL_GL_CreateContext"));
            }
            interval(1);
            let (mut w, mut h) = (0, 0);
            drawable(window, &mut w, &mut h);
            Ok(SdlSurface {
                lib,
                window,
                context,
                size: (w.max(1) as u32, h.max(1) as u32),
            })
        }
    }
}

impl Surface for SdlSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        let ok = unsafe {
            let make: unsafe extern "C" fn(Ptr, Ptr) -> c_int =
                sym(&self.lib, "SDL_GL_MakeCurrent")?;
            make(self.window, self.context)
        };
        match ok {
            0 => Ok(()),
            _ => Err(sdl_error(&self.lib, "SDL_GL_MakeCurrent")),
        }
    }

    fn window_size(&self) -> (u32, u32) {
        self.size
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        unsafe {
            let swap: unsafe extern "C" fn(Ptr) = sym(&self.lib, "SDL_GL_SwapWindow")?;
            swap(self.window);
        }
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(c) = CString::new(name) else {
            return std::ptr::null();
        };
        unsafe {
            sym::<unsafe extern "C" fn(*const c_char) -> *mut c_void>(
                &self.lib,
                "SDL_GL_GetProcAddress",
            )
            .map(|f| f(c.as_ptr()) as *const c_void)
            .unwrap_or(std::ptr::null())
        }
    }
}
```

Em `crates/slot-gfx/src/lib.rs`, acrescentar `mod sdl;` e `pub use sdl::SdlSurface;`.

- [ ] **Step 2: Usar como alternativa em `device_app.rs`**

Trocar a criação da superfície por uma `Box<dyn Surface>`:
```rust
    let mut surface: Box<dyn Surface> = match FbdevSurface::new() {
        Ok(s) => Box::new(s),
        Err(e) => {
            eprintln!("slot: {e}, trying SDL");
            match slot_gfx::SdlSurface::new() {
                Ok(s) => Box::new(s),
                Err(e) => {
                    eprintln!("slot: {e}");
                    return;
                }
            }
        }
    };
```
`Compositor::new(&surface)` passa a `Compositor::new(&*surface)`. Se a assinatura for genérica (`S: Surface`), usar `Compositor::new(surface.as_ref())`; confirmar a assinatura em `crates/slot-gfx/src/fbo.rs`.

- [ ] **Step 3: Verificar no Brick, registar em §14 e commit**

```bash
git add -A
git commit -m "Fall back to an SDL2 GLES surface when EGL refuses the Brick's framebuffer

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 17 (só se a §5 mostrou um daemon a reagir ao power ou ao volume): pausar daemons stock

**Files:**
- Modify: `card/Apps/Slot/launch.sh`, `card/test-launch.sh`

**Interfaces:**
- Consumes: os nomes dos processos em `docs/brick-hardware.md` §5/§13
- Produces: `launch.sh` que faz `killall -STOP` a esses processos antes do Slot e `killall -CONT` depois, mesmo que o Slot rebente

**Nunca** pôr `trimui_inputd` na lista: é ele que cria o gamepad.

- [ ] **Step 1: Teste em `card/test-launch.sh`**

Antes de `SLOT_SD=... sh "$TMP/app/launch.sh"`, acrescentar:
```sh
mkdir -p "$TMP/bin"
cat > "$TMP/bin/killall" <<-SH
	#!/bin/sh
	echo "\$*" >> "$TMP/killall.log"
SH
chmod +x "$TMP/bin/killall"
```
Mudar a chamada para:
```sh
PATH="$TMP/bin:$PATH" SLOT_PAUSE="keymon" SLOT_SD="$TMP/sd" SLOT_CORE_PATTERN="$TMP/core_pattern" sh "$TMP/app/launch.sh"
```
e acrescentar no fim:
```sh
[ "$(sed -n 1p "$TMP/killall.log")" = "-STOP keymon" ] \
	|| { echo "keymon was not paused before slot ran" >&2; exit 1; }
[ "$(sed -n 2p "$TMP/killall.log")" = "-CONT keymon" ] \
	|| { echo "keymon was not let go after slot" >&2; exit 1; }
```

```sh
sh card/test-launch.sh
```

Esperado: falha (`killall.log` não existe).

- [ ] **Step 2: Implementar no `launch.sh`**

Depois de `CORE_PATTERN=...`, acrescentar:
```sh
PAUSE="${SLOT_PAUSE-<nomes da §13 separados por espaço>}"
```
(substituir `<nomes ...>` pelo valor real; por exemplo `PAUSE="${SLOT_PAUSE-keymon}"`).

Antes de `cd "$APP"`:
```sh
for p in $PAUSE; do killall -STOP "$p" 2>/dev/null; done
```
Depois da linha que corre o `slot`:
```sh
for p in $PAUSE; do killall -CONT "$p" 2>/dev/null; done
```

```sh
sh card/test-launch.sh
```

Esperado: `launch: all passed`.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "Pause the stock key daemon while slot owns the buttons

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

---

### Task 18: Checklist completa no Brick, documentação e fecho

**Files:**
- Create: `README.brick.md`
- Modify: `CHANGELOG.md`, `docs/brick-hardware.md` (§15)

**Interfaces:**
- Consumes: tudo
- Produces: checklist da spec assinada pelo utilizador, guia de instalação e changelog

- [ ] **Step 1: Correr a checklist com o utilizador**

Para cada item, o utilizador diz OK ou descreve o problema. Os problemas corrigem-se antes de continuar: teste quando for lógica, e depois `task deploy:device`.

1. Imagem, moldura e 60 fps (Tarefa 16, Step 4).
2. Todos os botões, incluindo L2/R2: SELECT+L2/R2 troca a paleta GB com GB Palettes ligado; L1/R1 trocam de prateleira; X e Y fazem turbo.
3. Som na coluna e nos auscultadores; volume e mute (SELECT+VOL); o ícone de auscultadores aparece ao ligá-los.
4. Rumble num jogo com rumble (por exemplo Pokémon Pinball R&S, com rumble ligado no jogo).
5. LEDs F1/F2: verde a jogar, laranja ligado ao carregador; a barra de cima e as laterais apagadas.
6. Brilho e luz azul (combinações do menu do jogo).
7. Doze: toque curto no power apaga o ecrã e pausa; outro toque acorda e continua.
8. Exit to Menu: volta ao MainUI e repõe os LEDs, o brilho e o volume do sistema.
9. Start on Boot: ligar, reiniciar o Brick, confirmar que arranca no Slot e que Exit vai para o MainUI; desligar e reiniciar.
10. Reabrir o Slot retoma o último jogo.
11. Os dois cores: um jogo GBA, um GB e um GBC, em mGBA e em gpSP.
12. Save state, rewind e fast-forward.
13. Link entre dois Bricks, se houver dois. No fim, confirmar que o WiFi normal voltou (BrickTools → IP Address).

- [ ] **Step 2: Registar o resultado em `docs/brick-hardware.md` §15**

```markdown
## §15 Checklist final
| # | Item | Resultado |
|---|---|---|
| 1 | Imagem, moldura, 60 fps | |
| ... | ... | |
```

- [ ] **Step 3: Escrever `README.brick.md`**

Escrito para utilizadores do Brick, em português:

```markdown
# Slot no TrimUI Brick (StockMix)

## Instalar
1. Descarrega `dist-brick` (ou gera-o com `task dist:device`).
2. Copia as pastas `Apps/` e `Slot/` para a raiz do cartão SD do StockMix e aceita juntar as pastas.
3. As tuas ROMs ficam onde já estão: `Roms/GBA`, `Roms/GB` e `Roms/GBC`. Têm de estar descomprimidas.
4. Opcional: põe `gba_bios.bin` em `BIOS/` para a animação de arranque real.

## Usar
- Abre **Apps → Slot**.
- Manter o power abre o menu: **Exit to Menu** volta ao menu do StockMix.
- Toque curto no power: ecrã apaga e o jogo pausa.
- Menu → Gameplay → **Start on Boot** faz o Brick arrancar diretamente no Slot.

## Onde ficam as coisas
- Saves, save states, labels e definições: `Slot/` na raiz do cartão.
- Logs: `Slot/slot.log`. Se o Slot rebentar, em `Slot/crash/`.

## Atualizar
Substitui `Apps/Slot`. A pasta `Slot/` com os teus dados não é tocada.
```

- [ ] **Step 4: `CHANGELOG.md`**

No topo, acrescentar:
```markdown
## brick-1.0.0

- Runs on the TrimUI Brick under StockMix, as an app under Apps/Slot.
- Reads roms from the stock Roms/GBA, Roms/GB and Roms/GBC; saves, states, labels and settings live in Slot/.
- Exit to Menu in the power menu goes back to the StockMix menu and gives the LEDs, brightness and volume back.
- Start on Boot in the Gameplay settings starts the Brick straight into slot.
- Only the F1 and F2 button LEDs light up, in the SP's power LED colours.
- A short power press dozes, as closing the SP's lid did.
```

- [ ] **Step 5: Verificação final**

```sh
wsl -d Ubuntu-26.04 -- bash -lc 'cd /mnt/c/Users/lucas/Slot && task check && task dist:device'
```

Esperado: verde. Comparar a saída com a linha de base de `docs/brick-dev.md`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "docs: Brick install guide, changelog and the signed-off checklist

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>"
```

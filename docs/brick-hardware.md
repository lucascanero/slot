# TrimUI Brick: resultados da sonda (StockMix)

Sonda feita a 2026-10-11 por SSH (`tools/brick-probe/survey.sh`) e pela app `tools/brick-probe/SlotProbe`.

## §1 Sistema e glibc

```
Linux version 4.9.191 (TRIMUIDEV@NUC) (gcc version 7.4.1 (OpenWrt/Linaro GCC 7.4-2019.02 2019-02) ) #860 SMP PREEMPT Wed Nov 26 03:43:02 UTC 2025
/lib/ld-linux-aarch64.so.1 -> ld-2.33.so
/lib/libc.so.6 -> libc-2.33.so
Trimui Brick
```

- Tina Linux aarch64. Não tem `ldd`.
- **glibc do Brick:** 2.33  →  **BRICK_GLIBC a usar: 2.31**, que é menor ou igual.

## §2 Input

```
event0 name=sunxi-keyboard key=[c000000000000 0] abs=[0] sw=[0]
event1 name=axp2202-pek key=[10000000000000 0] abs=[0] sw=[0]
event2 name=audiocodec sunxi Audio Jack key=[40 0 0 0 0 0 400000000 0 c000000000000 0] abs=[0] sw=[14]
event3 name=TRIMUI Player1 key=[7cdb000000000000 0 0 c000000000000 1800000000000000] abs=[3003f] sw=[2]
```

Capacidades, descodificadas:

| Nó | Chaves | Eixos e switches |
|---|---|---|
| event0 `sunxi-keyboard` | 114, 115 (Vol−/Vol+) | — |
| event1 `axp2202-pek` | 116 (Power) | — |
| event2 `audiocodec sunxi Audio Jack` | 114, 115, 226, 582 | SW bits 2 (`SW_HEADPHONE_INSERT`) e 4 |
| event3 `TRIMUI Player1` (045e:028e, gamepad virtual do `trimui_inputd`) | 0x130, 0x131, 0x133, 0x134, 0x136, 0x137, 0x13a, 0x13b, 0x13c, 0x13d, 0x13e; 114, 115; 59, 60 | ABS X, Y, Z, RX, RY, RZ, HAT0X, HAT0Y; SW bit 1 |

**Nome do gamepad virtual:** `TRIMUI Player1`.

**Nós que repetem botões do gamepad:**
- Vol± aparecem em event0, event2 e event3.
- F1/F2 aparecem como 0x13d/0x13e e também como `KEY_F1`/`KEY_F2` (59/60) no event3.

Tabela observada: ver §2b.

## §3 GPU e ecrã

- `libEGL.so(.1)`, `libGLESv2.so(.2)`, `libGLES_CM`, `libIMGegl.so`, `libpvrNULL_WSEGL.so`: PowerVR com window system nulo.
- `libSDL2-2.0.so.0` (2.30.8) também existe.
- `fb0`: modo `U:1024x768p-60`, `virtual_size 1024,16384`, 32 bpp.
- `/dev/disp`: `crw------- 248,0`.

## §4 Áudio

- Uma só placa: `0 [audiocodec]`. O `aplay -L` lista `null` e `sysdefault:CARD=audiocodec`.
- Controlos relevantes, com os valores no momento da sonda:

| Controlo | Intervalo | Valor |
|---|---|---|
| `digital volume` | 0..63 | 30 |
| `DAC volume` | 0..255, 2 canais | 0,0 |
| `Soft Volume Master` | 0..255 | 255,255 |
| `Headphone Volume` | 0..7 | 1 |
| `Headphone Switch` | | on |
| `HpSpeaker Switch` | | on |
| `LINEOUT Switch` | | off |

- `/sys/class/speaker/mute` = 0.

## §5 Processos enquanto uma app corre

Com o BrickTools aberto, o MainUI **não** corre. A app é lançada assim:
```
{exe} ash /tmp/cmd_to_run.sh
{launch.sh} /bin/sh ./launch.sh
```

Daemons stock vivos:
- `keymon`, que tem abertos `event0`–`event3` e `js0`
- `trimui_inputd`
- `trimui_scened`
- `hardwareservice`
- `musicserver`
- `trimui_osdd`

`wpa_supplicant` e `udhcpc` também correm (WiFi ligado).

Reação do `keymon` ao power e ao volume dentro de uma app: ver §5b.

## §6 LEDs

- `/sys/class/led_anim` com efeitos por zona: `f1`, `f2`, `m`, `lr`, `l`, `r`.
- `effect_names`: `0 disable, 1 linear, 2 breath, 3 sniff, 4 static, 5 blink1, 6 blink2, …`. **O efeito 4 é estático.**
- `effect_rgb_hex_*` lê-se como `61A8DD` (hex sem `0x`, com um espaço no fim).
- `max_scale`: "maximum LED brightness [0 ~ tg5040 limit brightness 60]". Valores atuais: `max_scale=14`, `max_scale_f1f2=40`, `max_scale_lr=40`.
- O stock (`runtrimui.sh`) liga no arranque o efeito 1 a branco em `lr` e `m`.

Teste ao vivo: ver §6b.

## §7 Rumble

- `gpio227` já exportado pelo `runtrimui.sh`, como saída.
- `/sys/class/motor/voltage` = 3300000.

Teste ao vivo: ver §6b.

## §8 BIOS e ROMs

- Não existem `/mnt/SDCARD/BIOS` nem `/mnt/SDCARD/Bios`.
- `gba_bios.bin` está em:
  - `/mnt/SDCARD/RetroArch/.retroarch/system/gba_bios.bin`, que é candidato do `Layout`
  - `/mnt/SDCARD/Emus/GBA/gba_bios.bin`
- `Roms/` tem `GB`, `GBA`, `GBC` (e outras).
- `Roms/GBA` tem `.gba` soltos e pastas escondidas (`.games_config`, `.gpsp_libretro`, `.mgba_libretro`), que o scan ignora.

## §9 Arranque

`/etc/rc.d/S99runtrimui` → `/usr/trimui/bin/runtrimui.sh`, por esta ordem:
1. Exporta GPIOs (incluindo o 227 do motor).
2. Corre **em primeiro plano**, um a um, os `/mnt/SDCARD/System/starts/*.sh`.
3. `mkdir /tmp/trimui_inputd` e liga os LEDs `lr`/`m`.
4. Entra no ciclo principal:
   - `runifnecessary keymon/trimui_inputd/...` (a partir de `/mnt/UDISK/trimui`, porque `/mnt/SDCARD/trimui` não existe)
   - `premainui.sh`, `MainUI`, `preload.sh`
   - se existir `/tmp/cmd_to_run.sh`, corre-o e apaga-o
   - volta ao início.

**Consequência:** um script em `System/starts/` corre **antes** do `trimui_inputd`, logo sem gamepad. Correr o Slot aí não serve.

**Decisão do utilizador (opção A):** o script de arranque fica em fundo à espera do `MainUI`. Quando o MainUI aparece:
- escreve `/tmp/cmd_to_run.sh` a lançar o Slot;
- mata o `MainUI` com `-9`, e o ciclo trata o código 137 como sucesso;
- o ciclo corre o Slot, e ao sair volta ao MainUI.

Scripts já presentes em `System/starts/`: `ex_init.sh`, `sftpgo_boot.sh`, `sshd_boot.sh`.

## §10 WiFi

- Interfaces: `wlan0` e **`wlan1` já existe**. Chip `xradio`.
- `wpa_supplicant`, `wpa_cli`, `iw`, `ip`, `rfkill` e `hostapd` existem.
- Modos suportados: IBSS, managed, **AP**, AP/VLAN, monitor, P2P.
- Combinações válidas: `#{managed} <= 2, #{AP} <= 1, total <= 3, #channels <= 1`. AP e managed podem coexistir, **mas no mesmo canal**.
- Frequências: **só 2,4 GHz** (2412–2484 MHz, canais 1–14; 12–14 sem emissão). **Não há 5 GHz.**
- `wpa_supplicant` stock: `wpa_supplicant -B -D nl80211 -iwlan0 -c /etc/wifi/wpa_supplicant.conf -O /etc/wifi/sockets`, mais `udhcpc -i wlan0`.
- `ctrl_interface=/etc/wifi/sockets`.
- Não existe `/etc/wifi/wifi_init.sh`. O WiFi é ligado pelo próprio sistema (`systemval wifi`).

## §11 /run

`/run` existe e é gravável (`run-writable`).

## §12 Mounts

- `/` é `overlayfs` (rw) sobre `/rom` ext4 ro.
- O cartão está em `/mnt/SDCARD`.

## §13 Decisões que daqui resultam

- **Mapa de botões:** igual ao previsto na Tarefa 5 (0x130–0x13e no `TRIMUI Player1`). Aguarda a confirmação por evento na §2b.
- **BRICK_GLIBC:** 2.31.
- **Start on Boot:** opção A (§9). O script em `System/starts/slot-boot.sh` corre em fundo e entrega ao Slot por `/tmp/cmd_to_run.sh`.
- **Link por WiFi:**
  - Interface AP `wlan1`, que já existe.
  - Frequência **2,4 GHz**. O anfitrião usa a frequência a que o `wlan0` já está ligado, porque `#channels <= 1`; se o `wlan0` não estiver ligado, usa 2437.
  - ctrl stock `/etc/wifi/sockets`.
  - Restauro: relançar o `wpa_supplicant` como o stock faz, mais o `udhcpc`.
  - Com um RG SP, só funciona se o SP for o convidado: o SP anfitrião usa 5 GHz, que o Brick não ouve.
- **LEDs:** efeito 4 (estático); `max_scale_f1f2` a **40** (o valor do stock; o máximo é 60).
- **Daemons a pausar durante o Slot:** ver §5b. O `keymon` lê os mesmos nós.

## §2b Botões confirmados em uso

A app SlotProbe (dump evento a evento) não chegou a correr. O mapa foi confirmado de outra forma:
- as capacidades do `TRIMUI Player1` (§2);
- o primeiro arranque do Slot (§14): o utilizador navegou, meteu um cartucho e jogou com A/B/X/Y, L1/R1, D-pad e power sem problemas;
- os nós ficaram assim:

```
slot: input /dev/input/event3 (TRIMUI Player1) keys [130, 131, 133, 134, 136, 137, 13a, 13b, 13c, 73, 72] axes true jack false
slot: input /dev/input/event0 (sunxi-keyboard) keys [73, 72] axes false jack false
slot: input /dev/input/event1 (axp2202-pek) keys [74] axes false jack false
slot: input /dev/input/event2 (audiocodec sunxi Audio Jack) keys [73, 72] axes false jack true
```

Vol± chegam por três nós. O `Held` do input deixa passar cada clique uma só vez.

## §5b O `keymon` dentro de uma app

- Com o Slot aberto, o `keymon` stock continua vivo.
- Carregar em Vol± mostra o **pop-up de volume do sistema** por cima do HUD do Slot, relatado pelo utilizador.
- Também muda o mixer: depois de alguns cliques ficou `digital volume` = 24 e `DAC volume` = 0,0, quando o Slot tinha posto 0 e 160.
- Com `killall keymon`, o Slot continua a correr e o mixer deixa de ser mexido.
- O ciclo do `runtrimui.sh` volta a lançar o `keymon` (`runifnecessary`) antes do MainUI, por isso basta matá-lo enquanto o Slot corre: Tarefa 17.

## §6b LEDs e rumble em uso

No primeiro arranque o Slot reportou `motor gpio227, leds led_anim`. A verificação visual (F1/F2 verdes, barras apagadas, vibração) fica na checklist final (§15).

## §14 Primeiro arranque

O Slot foi aberto em Apps, a partir de `dist-brick` enviado por `task deploy:device`. Excerto do log:

```
slot: data /mnt/SDCARD/Slot, roms /mnt/SDCARD/Roms/GBA, app /mnt/SDCARD/Apps/Slot
slot: fb0 hint 1024x768
slot: audio plug:default at 32768 Hz
slot: panel 1024x768
slot: hardware volume taken: [("digital volume", "30"), ("DAC volume", "160,160")]
slot: backlight /dev/disp, battery axp2202-battery, rtc0, motor gpio227, leds led_anim
slot: bezel /mnt/SDCARD/Apps/Slot/bezels/1024x768.png
slot: boot: first frame at 3107.0 ms
slot: launch: opening Pokemon - Leaf Green Version (U) (V1.1) on mgba at 42585.0 ms
slot: core /mnt/SDCARD/Apps/Slot/mgba_libretro.so
slot: launch: first frame shown at 43352.2 ms
slot: audio: 12428 samples dropped, 53884 starved
```

- **Imagem:** o EGL do PowerVR aceitou a superfície fbdev à primeira, por isso a Tarefa 16b (SDL) não foi precisa.
- **O que funcionou:** imagem, moldura, botões, som, acerto da data e hora (`date`/`hwclock` ok), jogo GBA no mGBA.
- **Imagens dos cartuchos:** não apareciam, porque estão em `Imgs/<sistema>/` e não em `Slot/Labels/`. Passam a ser lidas daí.
- **A ver na checklist:** `samples dropped/starved` no fim da sessão; ouvir se há estalidos.

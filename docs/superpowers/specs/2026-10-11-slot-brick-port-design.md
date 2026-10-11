# Slot para TrimUI Brick (StockMix) — Design

Data: 2026-10-11
Estado: aprovado em conversa, por rever por escrito

## Objetivo

Portar o [slot](https://github.com/BrandonKowalski/slot) (frontend Rust de GB/GBC/GBA feito para o Anbernic RG SP sobre BaseOS) para o **TrimUI Brick** a correr **StockMix** (firmware stock da TrimUI com modificações), mantendo todas as funcionalidades do original.

O resultado é uma app que aparece no menu **Apps** do MainUI, com opção de arrancar diretamente no Slot ao ligar o Brick.

### Critérios de sucesso

1. A app abre a partir de Apps e mostra as prateleiras GBA/GB/GBC com as ROMs de `/mnt/SDCARD/Roms/`.
2. Os jogos correm a 60 fps com som, nos dois cores (mGBA e gpSP).
3. Todas as funcionalidades listadas em [Funcionalidades](#funcionalidades) funcionam no Brick.
4. "Sair" volta ao MainUI sem perder saves e com LEDs, brilho e volume repostos.
5. A opção "Start on boot" faz o Brick arrancar no Slot e pode ser desligada a partir do próprio Slot.
6. `cargo test --workspace` passa no WSL, e os testes novos cobrem a camada Brick.

## Decisões tomadas

| Tema | Decisão |
|---|---|
| Integração | App em `Apps/`, mais a opção configurável de arrancar no Slot |
| Pastas | ROMs e BIOS das pastas do StockMix; saves, states, labels e config numa pasta própria do Slot |
| Testes no hardware | SSH/ADB ao Brick a partir do PC |
| Estrutura do código | Fork só para Brick: o código Anbernic/BaseOS é removido |
| LEDs | Só os LEDs dos botões F1/F2; a barra superior e as barras laterais ficam desligadas |
| Boot logo | Fora de âmbito: vive numa partição do eMMC e escrevê-la arrisca o arranque |

## Fora de âmbito

- Suporte a Anbernic (RG SP, 34XX, 35XX, 28XX) e BaseOS.
- Brick Pro e Smart Pro (outros modelos com o mesmo SoC). Nada é feito para os impedir, mas não são alvo nem são testados.
- Leitura de ROMs em `.zip`/`.7z` (igual ao original).
- Partilha de `.sav` com o RetroArch do StockMix.
- Substituir o boot logo do Brick.
- Funções novas para F1/F2 (o original não as tem).

## Arquitetura

### Repositório

- `C:\Users\lucas\Slot` é um clone do original com o histórico completo. O remote `upstream` aponta para `BrandonKowalski/slot`, e o trabalho é no branch `brick`.
- Mantém-se a estrutura de crates: `slot`, `slot-gfx`, `slot-input`, `slot-power`, `slot-retro`, `slot-store`, `slot-ui` e `slot-sfxcut`.
- Mantém-se a build **host** (`--features host`), usada para testes e para correr a UI no PC.
- A build **device** (`--no-default-features --features device`) passa a visar o Brick.

### Removido

| O quê | Onde | Porquê |
|---|---|---|
| Tampa / sensor hall (`hallkey`, `SW_LID`) | `slot/src/input/device.rs`, `evdev.rs` | O Brick não tem tampa |
| Backlight por `dispdbg` e por `/sys/class/backlight` | `slot-power/src/device.rs` | O Brick usa `/dev/disp` |
| Rumble por evdev force-feedback | `slot-power/src/device.rs` | O Brick usa GPIO |
| Auscultadores por `/sys/kernel/debug/gpio` | `slot-power/src/device.rs` | O Brick usa `EV_SW` |
| LED `/sys/class/leds` | `slot-power/src/device.rs` | O Brick usa `/sys/class/led_anim` |
| Instalação do boot logo da BaseOS | `slot/src/bootlogo.rs` e respetivos testes | Fora de âmbito |
| `relink_adb` | `slot-power` e `app.rs` | Específico da BaseOS |
| Rotação para painel em retrato (`SLOT_ROTATE`, `set_panel_turn`) | `device_app.rs`, `slot-gfx` | Só para o RG28XX |
| `card/System/launch_frontend.sh` e `card/System/led.sh` (e `card/test-led.sh`) | `card/` | Arranque e LED da BaseOS; substituídos por `launch.sh` e pelo LED em Rust |
| `card/System/bootlogo.bmp` | `card/` | Boot logo fora de âmbito |

`coresave.sh` e `slotlink.sh` (com os seus testes) ficam e são adaptados: caminhos do Brick, e no link as alterações descritas em [Link por WiFi](#link-por-wifi-slotlinksh).
| Site de documentação, `CNAME`, workflow de pages | `site/`, `.github/workflows/pages.yml` | Não se aplica |

Ao remover, os testes que dependem destas peças são apagados ou adaptados. Os que testam lógica comum (por exemplo `read_only_in`, `uptime_seconds`) ficam.

### Toolchain e build

- **WSL Ubuntu**: `rustup` com o toolchain fixado em `rust-toolchain.toml` (1.96), mais `cargo-zigbuild` e `zig`.
- **Binário**: `cargo zigbuild --profile device --target aarch64-unknown-linux-gnu.2.31 -p slot --no-default-features --features device`.
  - A versão de glibc do alvo (2.31) tem de ser igual ou inferior à do Brick. Isto é confirmado no Brick (ver [Verificações prévias](#verificações-prévias-no-brick)) e ajustado se for preciso.
- **Cores**: `cores/mgba/build.sh` e `cores/gpsp/build.sh` com os patches do autor, compilados com `CC="zig cc -target aarch64-linux-gnu.2.31"`.
  - O source do gpSP continua a ser enviado com o pacote, por causa da GPL-2.0, secção 3(a).
- **Plano B**: se o zig falhar nos cores, usa-se um container `arm64` Debian bullseye com `qemu-user-static` dentro do WSL, igual ao processo do autor.
- **Tarefas novas/alteradas em `taskfile.yml`**:
  - `build:device`
  - `core:device`
  - `core:gpsp`
  - `dist:device`: gera `dist-brick/` com a árvore do SD
  - `deploy:device`: SSH (`scp`) ou ADB
  - `log:device`

## Camada de hardware do Brick

O Brick usa o SoC Allwinner A133P (aarch64), GPU PowerVR GE8300 e ecrã IPS 1024×768.

### Gráficos (`slot-gfx`)

- EGL e GLES2 por `libloading`, a partir de `libEGL.so(.1)` e `libGLESv2.so(.2)` do sistema. Isto não muda.
- Superfície: tenta a janela nativa fbdev e, se falhar, a janela nula. Ambos os caminhos já existem em `fbdev.rs`; o PowerVR deverá aceitar a janela nula.
  - Se nenhuma funcionar, investiga-se no Brick, por exemplo como o SDL2 stock cria o contexto. Isto é uma tarefa do plano, não uma suposição.
- O painel é 1024×768, um ecrã 4:3. Usa-se o modo **framed** existente:
  - O canvas 720×480 é escalado para 1024×683, encostado ao topo.
  - Por baixo fica a moldura do GBA SP.
- Adiciona-se `bezels/1024x768.png`, gerado a partir de `640x480.png` com escala de qualidade, sem alterar o desenho. O `pick()` já prefere a moldura de tamanho mais próximo.

### Input (`slot/src/input`)

Mapa de botões do gamepad TrimUI (do `trimui_inputd`, confirmado no Brick):

| Botão | Evento |
|---|---|
| A | `EV_KEY 0x131` |
| B | `EV_KEY 0x130` |
| X | `EV_KEY 0x134` |
| Y | `EV_KEY 0x133` |
| L1 | `EV_KEY 0x136` |
| R1 | `EV_KEY 0x137` |
| Select | `EV_KEY 0x13a` |
| Start | `EV_KEY 0x13b` |
| Menu | `EV_KEY 0x13c` |
| L2 | `EV_ABS ABS_Z` (0x02), pressionado acima do limiar |
| R2 | `EV_ABS ABS_RZ` (0x05), pressionado acima do limiar |
| D-pad | `EV_ABS ABS_HAT0X/ABS_HAT0Y` (já suportado por `Hat`) |
| Vol+ / Vol− | `EV_KEY 115 / 114` |
| Power | `EV_KEY 116` |
| F1 / F2 | `EV_KEY 0x13d / 0x13e`: ignorados |
| Auscultadores | `EV_SW SW_HEADPHONE_INSERT` (0x02) |

- O limiar de L2/R2 é metade do máximo do eixo, lido de `/sys/class/input/eventN/device/` ou do `EVIOCGABS`. Há histerese: solta abaixo de um quarto. Se o eixo reportar só 0/1, o limiar resulta em 1.
- Seleção de nós: lêem-se só os nós com as teclas pretendidas, como hoje. Se o gamepad virtual e o nó físico reportarem as mesmas teclas, lê-se só o gamepad, para não haver eventos duplicados. A regra exata (por nome do dispositivo) é definida depois de ver `/proc/bus/input/devices` no Brick.
- `Btn::Lid` e o polling de `hallkey` são removidos.
- O estado dos auscultadores passa a vir do input:
  - O `DeviceInput` mantém o último `SW_HEADPHONE_INSERT` num `AtomicBool` partilhado com a plataforma.
  - No arranque, o estado inicial vem de `EVIOCGSW`.

### Energia e periféricos (`slot-power`)

`DevicePlatform` é reescrito para o Brick, mantendo o trait `Platform`.

| Função | Implementação no Brick |
|---|---|
| `set_backlight(step)` | `ioctl(/dev/disp, DISP_LCD_SET_BRIGHTNESS = 0x102, [0, valor, 0, 0])`; o passo 0 (usado só pelo doze) dá 0 e apaga o ecrã; os passos 1..9 (a escala do utilizador) dão `[8, 16, 32, 48, 72, 96, 128, 176, 255]`, a curva do NextUI para o Brick |
| `battery()` / `charge()` | `power_supply` com `type == Battery` (o `axp2202-battery`); a deteção genérica atual mantém-se |
| `set_led(state)` | `/sys/class/led_anim`: efeitos desligados, cor estática nos LEDs F1/F2 via `effect_rgb_hex_f1/f2` ou `frame_hex` (o mecanismo é confirmado no Brick); `max_scale_lr` e o LED `m` a 0 |
| `set_rumble(strength)` | `/sys/class/motor/voltage` (escala entre 0,5 V e 3,3 V) e `/sys/class/gpio/gpio227/value` 1/0; exporta o GPIO 227 se ainda não estiver exportado |
| `headphones()` | Lê o `AtomicBool` alimentado pelo input |
| `card_read_only()` | Mantém-se (`/proc/mounts`) |
| `set_clock(secs)` | Mantém-se (`date` + `hwclock -w -u`) |
| `poweroff()` / `restart()` | `sync` e depois `poweroff` / `reboot` (comandos do sistema stock) |
| novo `exit_to_menu()` | Repõe LEDs, brilho e volume de hardware e termina o processo com código 0 |

Cores dos LEDs (iguais ao LED de energia do SP):

| Estado | Cor |
|---|---|
| A correr / carregado | verde `00FF00` |
| A carregar | laranja `FF8C00` |
| Bateria fraca | vermelho `FF0000` |
| Desligado | `000000` |

- **Estado anterior dos LEDs**: ao arrancar, guardam-se os valores de `max_scale`, `max_scale_lr`, `max_scale_f1f2` e dos ficheiros `effect_*` que o Slot vai alterar. São repostos em `exit_to_menu()`.
- **Brilho anterior**: lido com `DISP_LCD_GET_BRIGHTNESS` e reposto ao sair.

### Áudio (`slot/src/audio`)

- Saída ALSA por `libasound`, com a lista atual de dispositivos (`plug:default`, …). O volume em software mantém-se (`volume.rs`).
- Volume de hardware:
  - Ao arrancar, o Slot guarda o valor do controlo de volume de hardware e põe-no num nível fixo.
  - Ao sair, repõe o valor guardado.
  - Os nomes dos controlos (por exemplo `digital volume`, `DAC volume`) e o nível fixo são confirmados no Brick.
  - É feito com `amixer`, chamado como processo, tal como o NextUI faz.
- O original solta o dispositivo de áudio após 3 s de silêncio. Mantém-se.

### Botão power (substitui a tampa)

- **Toque curto**: "doze". O ecrã apaga e o jogo pausa, como ao fechar a tampa no SP. Outro toque curto acorda.
  - `Action::LidClose`/`LidOpen` deixam de existir; o doze passa a ser despoletado por `PowerTap`. O caminho `PowerTap` → `power_press()` já existe.
- **Manter**: abre o menu de energia (já existe).
- O desligar automático após o timeout em doze mantém-se.
- Se o sistema stock tiver um daemon que também reage ao power ou ao volume enquanto uma app corre, o Slot pára esse comportamento durante a sessão (por exemplo com `SIGSTOP` e `SIGCONT` ao sair) ou ignora as teclas em conflito. A decisão fica para depois da verificação no Brick.

### Link por WiFi (`slotlink.sh`)

O link não usa a rede WiFi a que o Brick está ligado. O `slotlink.sh` cria uma ligação direta entre as duas consolas:

- **Anfitrião**: levanta uma rede própria (`ssid slotlink`, WPA2, 5745 MHz) com `wpa_supplicant` em modo AP na interface `wlan1`, com o IP 10.42.0.1.
- **Convidado**: pára o `wpa_supplicant` de `wlan0`, liga-se a essa rede e fica com o IP 10.42.0.2.
- O protocolo de jogo por cima (`link_net.rs`, porta 7211) não muda.

Adaptações no Brick:

1. **Interface AP.**
   - Se o driver WiFi do Brick não criar `wlan1`, o script cria-a com `iw dev wlan0 interface add wlan1 type __ap`.
   - Se o driver não suportar AP e STA ao mesmo tempo, usa `wlan0` em modo AP.
   - A escolha é feita depois da verificação no Brick.
2. **Frequência.** Se o Brick não puder ser AP em 5745 MHz (canal 149), usa-se um canal suportado pelo chip, igual nos dois lados. Em 2,4 GHz, o canal 6 (2437 MHz).
3. **Repor o WiFi do sistema.** O original não restaura a rede a que a consola estava ligada. No StockMix, `link down` e a saída do Slot voltam a ligar o WiFi stock, com o mesmo comando que o sistema usa no arranque (confirmado no Brick, por exemplo `/etc/wifi/wifi_init.sh start` ou relançar o `wpa_supplicant` com `/etc/wifi/wpa_supplicant.conf`).
4. **Caminhos.** O script passa a viver em `Apps/Slot/slotlink.sh`, chamado através de `Layout::app()`, e `RUN` aponta para `/tmp` se `/run` não existir.

Os testes de cartão do autor (`card/test-slotlink.sh`) são mantidos e alargados aos casos novos: criação de `wlan1` e reposição do WiFi stock.

## Integração com o StockMix

### Pastas

```
/mnt/SDCARD/
├── Apps/Slot/                 pacote da app (substituído ao atualizar)
│   ├── config.json            {"label":"Slot","icon":"icon.png","launch":"launch.sh","description":"Game Boy shelf"}
│   ├── launch.sh
│   ├── icon.png
│   ├── slot                   binário aarch64
│   ├── mgba_libretro.so
│   ├── gpsp_libretro.so
│   ├── bezels/1024x768.png
│   ├── boot/slot-boot.sh      script instalado pela opção "Start on boot"
│   └── licenses/
├── Slot/                      dados do utilizador (sobrevivem a updates)
│   ├── Config/                slot.state, settings, theme.txt, cart_shell.txt, wallpaper.deck, ...
│   ├── Saves/{GBA,GB,GBC}/
│   ├── States/{GBA,GB,GBC}/
│   ├── Labels/{GBA,GB,GBC}/
│   ├── Wallpapers/
│   ├── BIOS/                  alternativa a gba_bios.bin
│   ├── crash/
│   ├── slot.log, slot.log.1
└── Roms/{GBA,GB,GBC}/         só leitura para o Slot
```

### Resolução de caminhos

- Hoje o código usa um único `root` com subpastas fixas (`root.join("Games")`, `root.join("System")`, …). Isto passa para uma struct `Layout` em `slot-store`, com um método por pasta:
  - `roms(platform)`
  - `saves(platform)`
  - `states(platform)`
  - `labels(platform)`
  - `config()`
  - `wallpapers()`
  - `bios_candidates()`
  - `app()`, para cores e molduras
  - `data()`
- `Layout::stockmix(sd: &Path, app: &Path)` constrói as pastas acima. Os testes usam o mesmo construtor com uma pasta temporária.
- **BIOS**: procura `gba_bios.bin` por ordem em:
  1. a pasta de BIOS do StockMix (caminho confirmado no Brick; candidatos: `/mnt/SDCARD/BIOS`, `/mnt/SDCARD/Bios`, `/mnt/SDCARD/RetroArch/.retroarch/system`)
  2. `Slot/BIOS/`
- `--init-root` passa a criar só a árvore `Slot/`, sem criar nem mexer em `Roms/`.
- Variáveis de ambiente para desenvolvimento: `SLOT_SD` (por omissão `/mnt/SDCARD`) e `SLOT_APP` (por omissão a pasta do binário). Substituem `SLOT_ROOT`.

### `launch.sh`

```sh
#!/bin/sh
APP="$(cd "$(dirname "$0")" && pwd)"
SD=/mnt/SDCARD
DATA="$SD/Slot"
mkdir -p "$DATA"
[ -f "$DATA/slot.log" ] && mv -f "$DATA/slot.log" "$DATA/slot.log.1"
OLD_CORE="$(cat /proc/sys/kernel/core_pattern 2>/dev/null)"
echo "|/bin/sh $APP/coresave.sh %p %s %e %t" > /proc/sys/kernel/core_pattern 2>/dev/null
export SLOT_SD="$SD" SLOT_APP="$APP"
cd "$APP"
"$APP/slot" > "$DATA/slot.log" 2>&1
[ -n "$OLD_CORE" ] && echo "$OLD_CORE" > /proc/sys/kernel/core_pattern 2>/dev/null
exit 0
```

- O `core_pattern` só é trocado durante a sessão e é reposto ao sair, para não afetar outras apps.
- O `coresave.sh` passa a usar `$SLOT_SD/Slot/` (crash e log) em vez da raiz do cartão.
- O loop do sistema stock volta a lançar o MainUI quando o Slot termina.

### Ciclo de vida

- **Abrir**: o Slot retoma o último jogo, se o havia, como hoje no arranque.
- **Menu de energia**: `PowerChoice` passa a ser `ExitToMenu`, `Restart`, `PowerOff`, por esta ordem. "Exit" mostra a mesma animação de saída que "Restart". Antes de sair, grava sempre o jogo (`settle_saves`) e guarda o estado de retoma.
- **Start on boot**:
  - É uma nova linha no menu rápido, entre os settings de Gameplay. Os valores são Off e On, com wrap como os restantes.
  - On copia `Apps/Slot/boot/slot-boot.sh` para o diretório de scripts de arranque do StockMix. Off remove-o.
  - O script lança o Slot em primeiro plano antes de o MainUI aparecer. Ao sair do Slot, o arranque continua normalmente para o MainUI.
  - O estado mostrado no menu é a existência do ficheiro, não uma cópia guardada noutro sítio.
  - O mecanismo exato (pasta, primeiro plano ou fundo, ordem face ao MainUI) é confirmado no Brick antes de ser implementado. Se `System/starts/` correr em fundo, usa-se outro ponto do arranque stock, também confirmado no Brick.

## Funcionalidades

Mantêm-se todas as do original (CHANGELOG até 1.6.0):

- Prateleiras GBA, GB e GBC. L1/R1 trocam de prateleira; cima/baixo saltam por letra.
- Cartuchos com labels, cores de concha por cartucho (`cart_shell.txt`) e cache de labels.
- Inserção com som e animação.
- Escolha de core por cartucho: mGBA ou gpSP (START abre o cartucho).
- Animação da BIOS real, quando existe.
- Save states com seletor, rewind e fast-forward com velocidades.
- Turbo: X para turbo A, Y para turbo B.
- Menu rápido com:
  - velocidade de FF
  - som
  - rumble
  - correção de cor
  - data e hora
  - About
  - páginas Screen e Gameplay
- Shaders por plataforma (Off, LCD3x, Grid, Dot / Off, Grid, Simpletex), com SELECT+B/A no jogo.
- Paletas GB nomeadas, com SELECT+L2/R2.
- Esticar GB com L1/R1.
- Brilho e luz azul.
- Volume e mute separados para coluna e auscultadores, mute com SELECT+VOL.
- Auto-save ao ejetar; turbo e rewind desligáveis.
- RTC de cartucho GB no `.sav` e RTC GBA a correr no gpSP.
- Rumble, incluindo Pokémon Pinball R&S.
- Wallpapers em baralho e polaroids.
- Alarme de cartão só de leitura ou save falhado.
- Logs de crash em `crash/`.
- Link por WiFi: cabo link, Wireless Adapter, link mode do mGBA e link GB.
  - Alvo: entre dois Bricks.
  - Com um RG SP a correr o Slot original deve funcionar se o protocolo coincidir, mas não é garantido nem testado.
  - Ver [Link por WiFi](#link-por-wifi-slotlinksh) para o que muda.

Alterado:

- "Fechar a tampa" passa a toque curto no power.
- O LED de energia passa a LEDs F1/F2.
- "Retomar no arranque" passa a "retomar ao abrir" (e no arranque, com Start on boot).
- Novo: Exit to menu e Start on boot.

## Testes

### Automáticos (WSL, `cargo test --workspace`)

- A suite existente continua a passar, à exceção dos testes de peças removidas, que são apagados com elas.
- Testes novos, no estilo do autor (sysfs e `/dev` falsos em `tempfile`):

| Área | O que testa |
|---|---|
| `input/evdev.rs` | mapa de botões TrimUI; L2/R2 por `ABS_Z`/`ABS_RZ` com limiar e histerese; `SW_HEADPHONE_INSERT`; F1/F2 ignorados |
| seleção de nós | sem duplicados quando o gamepad virtual e o nó físico têm as mesmas teclas |
| `slot-power` LEDs | ficheiros e valores escritos em `led_anim` por estado; o estado anterior é guardado e reposto |
| `slot-power` rumble | `voltage` e `gpio227/value` com a escala correta; liga/desliga só nas transições |
| `slot-power` brilho | mapeamento passo→valor bruto (função pura; o ioctl fica fora do teste) |
| `Layout` | todas as pastas para StockMix; ordem de procura da BIOS; `--init-root` não toca em `Roms/` |
| Start on boot | instala, remove e reporta o estado pelo ficheiro |
| `PowerChoice` | Exit é a primeira opção; Exit grava antes de sair |

### No Brick (por SSH/ADB)

- `task deploy:device` envia o pacote; `task log:device` lê o `slot.log`.
- Checklist manual, corrida em conjunto com o utilizador:
  1. Imagem, moldura e 60 fps.
  2. Todos os botões, incluindo L2/R2.
  3. Som na coluna e nos auscultadores; volume e mute.
  4. Rumble.
  5. LEDs F1/F2 nos estados a correr, a carregar e bateria fraca.
  6. Brilho e luz azul.
  7. Doze e acordar.
  8. Exit to menu volta ao MainUI com tudo reposto.
  9. Start on boot: ligar e desligar.
  10. Retoma ao reabrir.
  11. Os dois cores com um jogo GBA, GB e GBC.
  12. Save state, rewind e FF.
  13. Link entre dois Bricks, se houver dois.

## Verificações prévias no Brick

São feitas por SSH antes de escrever o código que delas depende. Os resultados ficam num ficheiro `docs/brick-hardware.md`.

1. `ldd --version` / `/lib/libc.so.6`: versão da glibc.
2. `cat /proc/bus/input/devices` e `evtest` (ou `cat` de `eventN` com hexdump) para confirmar códigos, eixos de L2/R2 e `SW_HEADPHONE_INSERT`.
3. `ls /usr/lib | grep -i -E 'egl|gles|pvr'`: bibliotecas da GPU.
4. `amixer scontrols` / `amixer`: controlos de volume.
5. `ps` com uma app a correr: daemons stock que tratam de power/volume.
6. `ls /sys/class/led_anim`: ficheiros dos LEDs.
7. `ls /sys/class/motor /sys/class/gpio`: rumble.
8. Pasta de BIOS do StockMix e onde está o `gba_bios.bin`, se existir.
9. Scripts de arranque stock (`/etc/init.d/`, `runtrimui.sh` ou equivalente): como e quando correm os `System/starts/*.sh` e onde ligar o "Start on boot".
10. WiFi:
    - `iw list`: modos suportados (AP, combinações AP+STA) e frequências.
    - `ls /sys/class/net`: existe `wlan1`?
    - `which wpa_supplicant wpa_cli iw ip rfkill`.
    - Como o sistema stock liga o WiFi no arranque.
11. `/run` existe e é gravável? Se não, usa-se `/tmp`.

## Riscos

| Risco | Mitigação |
|---|---|
| O EGL do PowerVR não aceita nenhum dos caminhos de superfície atuais | Investigar o que o SDL2 stock usa; tarefa dedicada no plano |
| glibc do Brick mais antiga que 2.31 | Baixar o alvo do zigbuild para a versão do Brick |
| Daemons stock a reagir às mesmas teclas | Pausar o daemon durante a sessão ou ignorar a tecla em conflito, conforme a verificação |
| `zig cc` não compila um dos cores | Plano B: container arm64 bullseye com qemu |
| Desempenho do gpSP/mGBA no A133P diferente do H700 | Os dois SoCs são Cortex-A53 a frequências semelhantes; medir com `core_bench` no Brick |
| O chip WiFi do Brick não suporta modo AP | Sem AP não há anfitrião: o link fica indisponível e o Slot mostra a recusa que já existe para falhas de rádio. Tudo o resto funciona. O utilizador é avisado antes de se escrever mais código de link |
| Depois do link, o Brick fica sem a rede WiFi normal | Reposição explícita do WiFi stock em `link down` e na saída (ver Link por WiFi) |

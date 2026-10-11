# Slot no Brick: modo "Use RetroArch" — Design

Data: 2026-10-12
Estado: aprovado em conversa, por rever por escrito
Base: branch `brick` (port do Slot para TrimUI Brick / StockMix já a correr no Brick)

## Objetivo

Uma opção **Use RetroArch** no Slot. Com ela ligada:
- O Slot continua a ser a prateleira de cartuchos.
- Cada jogo abre no **RetroArch do StockMix**, com os saves, os save states, o menu e o **cloud sync** do RetroArch.
- Ao sair do RetroArch volta-se à prateleira do Slot.
- O **link entre dois Bricks** também funciona neste modo, pelo netplay do RetroArch.

### Critérios de sucesso

1. Com a opção ligada, meter um cartucho abre o jogo no RetroArch com o core escolhido no Slot (mGBA ou gpSP; GB e GBC usam o mGBA).
2. Os saves desse jogo ficam nos saves do RetroArch, e o cloud sync do RetroArch corre como num jogo aberto pelo StockMix.
3. Sair do RetroArch volta à prateleira do Slot, com o mesmo cartucho em frente, em poucos segundos.
4. No RetroArch, volume, brilho e adormecer funcionam como num jogo lançado pelo StockMix.
5. O menu de opções troca as páginas Screen e Gameplay por **RetroArch Settings**, que abre o menu do RetroArch e volta ao Slot.
6. Com dois Bricks, **Host Link** num e **Join Link** no outro ligam o mesmo jogo pelo netplay do RetroArch, sem router.
7. Com a opção desligada, o Slot comporta-se exatamente como hoje.
8. `cargo test --workspace --lib --bins --tests` e os testes de cartão passam.

## Decisões tomadas

| Tema | Decisão |
|---|---|
| Saves | **Mundos separados**: os do Slot (`Slot/Saves`) e os do RetroArch nunca se tocam |
| Core | O escolhido no Slot (START no cartucho: mGBA ou gpSP; GB/GBC no mGBA). O ficheiro é o do próprio Slot (`Apps/Slot/*_libretro.so`), com os patches de link, carregado pelo RetroArch |
| Passagem do ecrã | Pelo `launch.sh`: o Slot escreve um pedido e sai, o `launch.sh` corre o RetroArch e depois reabre o Slot |
| Menu de opções | Screen e Gameplay são substituídas por **RetroArch Settings**. Ficam: Use RetroArch, Start on Boot, Date & Time, Download Labels, About, Exit to Menu |
| Link | Como no Slot: **Host Link** / **Join Link** criam a rede WiFi direta (`slotlink.sh`) e abrem o RetroArch já em `--host` / `--connect 10.42.0.1` |
| Cloud sync | O do RetroArch, no início e no fim de cada jogo. Não há sincronização própria no arranque do Slot |

## Fora de âmbito

- Copiar ou fundir saves entre os dois modos.
- Mudar o `retroarch.cfg` do utilizador (incluindo `config_save_on_exit`).
- Usar os cores do RetroArch do StockMix (`RetroArch/.retroarch/cores`) neste modo.
- Os "folder overrides" do StockMix (`FolderOverrideFinder.sh`).
- Link com um RG SP (a passagem de link usa a mesma rede do Slot, mas o protocolo é o netplay do RetroArch, não o do Slot).
- Escolher no Slot outros cores do RetroArch (VBA-M, Gambatte, …).

## Arquitetura

### Peças

| Peça | Onde | Responsabilidade |
|---|---|---|
| Estado `use_retroarch` | `slot-store/src/slot_state.rs` | Opção gravada no `slot.state` (falso por omissão; ficheiros antigos leem-se como falso) |
| `RaRequest` | novo `crates/slot/src/retroarch.rs` | O pedido de passagem: core, ROM, link (`host`/`join`/nenhum) ou "abrir só o menu". Escrito e lido num ficheiro de texto simples |
| App | `crates/slot/src/app.rs` | No modo RetroArch: inserir um cartucho, RetroArch Settings ou um link escolhido geram um pedido e a saída da app |
| Menu | `crates/slot-ui/src/quick_menu.rs`, `app.rs` | Linhas **Use RetroArch** e **RetroArch Settings**; páginas conforme o modo |
| Ecrã de link na prateleira | `app.rs` + UI de link existente | Com o modo RetroArch ligado, **SELECT** sobre um cartucho abre a escolha HOST / JOIN, com a arte de link que o Slot já tem |
| Rádio | `link_radio.rs` (existente) | Levanta a rede direta antes da passagem (`slotlink.sh link host|join`) |
| `retroarch.sh` | novo `card/Apps/Slot/retroarch.sh` | Prepara o ambiente do StockMix e corre `ra64.trimui` com o core, a ROM e os argumentos de netplay |
| `launch.sh` | `card/Apps/Slot/launch.sh` | Ciclo: Slot → (pedido?) → `keymon` de volta → RetroArch → `keymon` fora → link em baixo → Slot |

### Fluxo

```
launch.sh
 ├─ guarda como o keymon estava a correr (exe, pasta, LD_LIBRARY_PATH) e mata-o
 └─ ciclo:
     ├─ corre o slot
     ├─ se não houver Slot/Config/retroarch.request → sai do ciclo (Exit to Menu, Power Off, …)
     ├─ lê o pedido e apaga-o
     ├─ relança o keymon tal como estava
     ├─ sh retroarch.sh <core> <rom|-> <host|join|->
     ├─ mata o keymon
     ├─ se o pedido tinha link → sh slotlink.sh link down
     └─ volta ao início (reabre o slot)
```

**O pedido** (`Slot/Config/retroarch.request`):
- Três linhas, `core=…`, `rom=…` e `link=…`. A `rom` pode estar vazia ("só o menu") e o `link` também.
- É escrito com a escrita atómica que o Slot já usa (`atomic_write`), por isso um pedido meio escrito nunca é lido.

**Ao sair para o RetroArch**, o Slot:
- grava o estado com o cartucho em frente;
- **não** liberta o hardware com `release()`, porque o Slot volta já a seguir e o `launch.sh` trata do `keymon`;
- termina o processo como no "Exit to Menu".

**Ao voltar**, o Slot arranca normalmente e mostra a prateleira com esse cartucho em frente. No modo RetroArch, o "retomar o jogo" do Slot não se aplica: o jogo estava no RetroArch.

### `retroarch.sh`

Reproduz o que os scripts `Emus/<sistema>/*.sh` do StockMix fazem antes de lançar o RetroArch, sem depender do caminho em `Emus/`:

```sh
export PATH="/mnt/SDCARD/System/usr/trimui/scripts:/mnt/SDCARD/System/bin:$PATH"
export LD_LIBRARY_PATH="/usr/trimui/lib:/mnt/SDCARD/System/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
RA_DIR=/mnt/SDCARD/RetroArch
ra_audio_switcher.sh
touch /var/trimui_inputd/ra_hotkey
cpufreq.sh ondemand 2 6
cd "$RA_DIR"
HOME="$RA_DIR" ./ra64.trimui -v -L "$core" [--host | --connect 10.42.0.1] "$rom"
```

- **Só o menu** (RetroArch Settings): o mesmo, sem `-L` e sem ROM.
- **Caminhos:** todos os caminhos e programas podem ser trocados por variáveis de ambiente, para os testes de cartão.
- **Saída:** devolve o código do RetroArch. O `launch.sh` volta ao Slot com qualquer código.

### Menu de opções

| Modo | Página principal |
|---|---|
| Slot | Screen · Gameplay · Date & Time · About · Download Labels · **Use RetroArch** · Exit to Menu |
| RetroArch | **Use RetroArch** · **RetroArch Settings** · Start on Boot · Date & Time · About · Download Labels · Exit to Menu |

- As duas páginas têm 7 linhas (hoje são 6). Cabem no ecrã por cima da legenda: 7 × 52 px = 364 px.
- Com o modo RetroArch ligado, as páginas Screen e Gameplay não aparecem. Os valores delas ficam gravados para quando o modo é desligado.
- No modo Slot, Start on Boot continua na página Gameplay, como hoje.
- **Use RetroArch:** On/Off com esquerda/direita, como as outras linhas. A página muda logo e a barra de seleção acompanha a linha Use RetroArch para a sua nova posição.
- **RetroArch Settings:** A gera um pedido "só o menu".

### Link no modo RetroArch

- **SELECT** sobre um cartucho, só no modo RetroArch, abre o ecrã HOST / JOIN do Slot (`GameMenu::Pick`, o mesmo que o Slot abre a meio do jogo) para esse cartucho e com o core escolhido para ele. B volta à prateleira. Na prateleira, o SELECT não faz nada hoje.
- **Mesmas regras do Slot** para poder abrir (as de `open_game_menu`, aplicadas ao cartucho em frente e não a um cartucho a correr):
  - com o mGBA abre o link por cabo, exceto nos jogos GBA sem fios, que mostram `NeedsGpsp`;
  - com o gpSP só abre em jogos GBA com link conhecido (`link_carried`), com cabo ou sem fios conforme o jogo; os outros mostram `NoLink`;
  - "o core" aqui é o escolhido para o cartucho em frente, não um core a correr.
- **HOST:**
  1. O rádio corre `slotlink.sh link host`, com o ecrã "Working" que já existe.
  2. Quando a rede está em cima, o Slot escreve o pedido com `link=host` e sai.
  3. O RetroArch abre com `--host`.
- **JOIN:**
  1. O rádio corre `slotlink.sh link join`.
  2. Quando liga, o Slot escreve o pedido com `link=join` e sai.
  3. O RetroArch abre com `--connect 10.42.0.1`.
  4. Se não encontrar anfitrião, mostra a recusa de link que já existe e fica na prateleira.
- **No fim:** o `launch.sh` corre `slotlink.sh link down`, que desfaz a rede direta e repõe o WiFi stock.

### Erros

| Situação | Comportamento |
|---|---|
| Falta `ra64.trimui` ou o core do Slot | O Slot não gera o pedido, mostra a recusa existente (`Refusal`) e fica na prateleira. O motivo vai para o log |
| O RetroArch rebenta ou sai com erro | O `launch.sh` volta ao Slot na mesma |
| Pedido corrompido ou ilegível | O `launch.sh` ignora-o, apaga-o e volta ao Slot |
| O Brick adormece dentro do RetroArch | É o `keymon` do StockMix que trata, como em qualquer jogo |
| O rádio falha no Host/Join | A recusa de link existente; sem pedido |

## Verificação prévia no Brick (primeira tarefa do plano)

Antes de construir o resto, confirmar no Brick, por SSH e com o utilizador:
1. `ra64.trimui -L /mnt/SDCARD/Apps/Slot/gpsp_libretro.so "<rom>"` abre o jogo com o ambiente do `retroarch.sh`, com som, botões e hotkeys do StockMix.
2. O log verboso do RetroArch mostra o core a registar a interface netpacket (o link é possível) e o `--host` a escutar.
3. Os saves vão para `RetroArch/.retroarch/saves`.
4. Com dois Bricks, se houver: `--host` num e `--connect` no outro, numa troca Pokémon com o gpSP.

Se o ponto 1 ou 2 falhar, o utilizador é avisado antes de se escrever mais código.

## Testes

### Automáticos

| Área | O que testa |
|---|---|
| `RaRequest` | Escreve e lê de volta core, ROM e link; pedido "só o menu"; ficheiro corrompido lê-se como nenhum pedido |
| Estado | `use_retroarch` grava e lê; ficheiros antigos sem a chave leem-se como falso |
| App, modo RetroArch | Inserir gera o pedido com o core certo (GBA mGBA/gpSP, GB/GBC mGBA) e a app sai; com o core em falta, recusa e fica na prateleira |
| App, modo Slot | Inserir continua a abrir o jogo no motor do Slot (os testes existentes continuam verdes) |
| Menu | Linhas e páginas conforme o modo; RetroArch Settings gera o pedido "só o menu" |
| Link | SELECT abre HOST/JOIN só no modo RetroArch; rádio bem-sucedido gera o pedido com o link; rádio falhado não |
| `launch.sh` (cartão) | Com um Slot e um RetroArch falsos: sem pedido sai do ciclo; com pedido corre o RetroArch, relança e mata o `keymon` à volta dele, desfaz o link e volta ao Slot; um RetroArch que falha volta ao Slot na mesma |
| `retroarch.sh` (cartão) | Argumentos certos para jogo, só menu, host e join; ambiente do StockMix exportado |

### No Brick

Checklist com o utilizador:
1. Jogo GBA no mGBA e no gpSP pelo RetroArch.
2. Jogo GB.
3. Save gravado e visto no cloud sync.
4. Sair volta à prateleira com o cartucho em frente.
5. RetroArch Settings abre e volta.
6. Volume, brilho e adormecer dentro do RetroArch.
7. Desligar o modo volta ao comportamento atual.
8. Link entre dois Bricks, se houver dois.

## Riscos

| Risco | Mitigação |
|---|---|
| O `ra64.trimui` do StockMix não carrega o core do Slot | Verificação prévia (ponto 1); o core do Slot é o mesmo mGBA/gpSP do libretro, compilado para a mesma glibc |
| O netplay do RetroArch não transporta o link dos cores do Slot | Verificação prévia (ponto 2); se falhar, o link no modo RetroArch fica fora e o resto segue |
| O `keymon` relançado pelo `launch.sh` não fica igual ao do StockMix | O `launch.sh` relança-o com o mesmo executável, pasta e `LD_LIBRARY_PATH` que tinha; a checklist verifica volume, brilho e adormecer no RetroArch |
| Arranque do Slot ao voltar (~2 s) | Aceite (abordagem A); o arranque já demora cerca de 1–2 s no Brick |

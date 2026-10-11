# Desenvolvimento do Slot para TrimUI Brick

Tudo corre no WSL (`wsl -d Ubuntu-26.04`), a partir de `/mnt/c/Users/lucas/Slot`.

O `wsl.exe` volta a interpretar as aspas da linha de comandos, e o Git Bash reescreve caminhos `/mnt/...`. Por isso os comandos longos ficam num script e correm assim, a partir do PowerShell:
```
wsl -d Ubuntu-26.04 -- bash /mnt/c/.../script.sh
```
O script deve pôr o `PATH` assim:
```
PATH=$HOME/.local/bin:$HOME/.cargo/bin:/usr/bin:/bin
```

## Ferramentas

- **Rust:** o toolchain de `rust-toolchain.toml` (1.96.1) e `rustup target add aarch64-unknown-linux-gnu`.
- **zig 0.14.1:** em `~/.local/opt`, ligado a partir de `~/.local/bin/zig`.
- **`cargo-zigbuild`:** 0.23.4, instalado com `cargo install cargo-zigbuild --locked`.
- **go-task:** 3.54.0, em `~/.local/bin/task`.
- **apt:**
  ```
  build-essential pkg-config libasound2-dev libudev-dev cmake imagemagick librsvg2-bin binutils openssh-client
  ```
- **SSH ao Brick:**
  - alias `brick` em `~/.ssh/config` (root@192.168.1.72);
  - a chave do WSL está em `/root/.ssh/authorized_keys` no Brick. Para a tirar, apaga essa linha.

## Comandos de verificação

O autor desenvolve em macOS. No Linux, os exemplos (`crates/slot/examples/record.rs`) e os testes de renderização só compilam no macOS, por isso testes e clippy correm sem exemplos:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --lib --bins --tests -- -D warnings
cargo clippy -p slot --no-default-features --features device -- -D warnings
cargo test --workspace --lib --bins --tests --no-fail-fast
sh card/test-*.sh
```

## Linha de base (antes do port, commit e7f5af5)

Primeiro foi preciso:
- acrescentar `#![cfg(target_os = "macos")]` a `crates/slot-gfx/tests/pipeline.rs`, como os outros testes de renderização;
- formatar e tirar um `mut` a mais em `crates/slot/tests/prefetch.rs`.

Com isso, `fmt`, `clippy` (host e device) e os testes de cartão passam.

Dos testes Rust, 142 binários passam. Falham 9 testes, sempre e sem relação com o port:

| Binário | Testes | Causa |
|---|---|---|
| `slot --test sfx` | `a_cart_sound_is_played_at_the_volume_that_is_set`, `a_clip_plays_with_no_core_running` | O WSL não tem dispositivo de áudio ("The requested device is no longer available") |
| `slot --test emu` | `a_present_runs_what_it_can_afford_rather_than_the_whole_ceiling` | O orçamento do fast-forward deixa correr 4,7 frames por apresentação no Linux (o teste aceita 4). Verificar o fast-forward no Brick |
| `slot --test link_ui` | `a_in_the_mode_the_game_already_runs_starts_the_link_straight_away`, `a_link_in_a_switched_mode_reloads_the_game_and_then_starts_the_link`, `a_reloads_only_for_a_serial_the_core_was_not_loaded_with`, `a_switch_is_refused_over_a_resume_the_core_would_not_take`, `select_is_refused_where_gpsp_would_link_the_same_either_way`, `the_reload_finishing_starts_the_link` | Cancelar um link à espera (B em "Working") não termina no WSL dentro do prazo do teste, também em série. Verificar o cancelamento no Brick |

Estas falhas não contam para "verde" nas tarefas do port. Qualquer outra falha conta.

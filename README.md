# wlptrctl

Small Wayland virtual-pointer control daemon and client. Emits scroll, motion,
and button events through `zwlr_virtual_pointer_v1`.
## Build

```sh
cargo build --release
```

## Install

```sh
cargo install --path .
```

## Daemon

Run once inside the Wayland session:

```sh
wlptrctl daemon
```

It listens on `$XDG_RUNTIME_DIR/wlptrctl.sock`.

## Usage

```sh
wlptrctl scroll <vertical-steps> <horizontal-steps>
wlptrctl motion <dx> <dy>
wlptrctl button <left|right|middle|NUM> <press|release>
```

Examples:

```sh
wlptrctl scroll 1 0
wlptrctl scroll -1 0
wlptrctl scroll 0 1
wlptrctl motion 50 0
wlptrctl button left press
wlptrctl button left release
wlptrctl click left
```

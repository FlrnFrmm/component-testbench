# Component

## Prerequisites

```sh
cargo install cargo-component --locked
```

```sh
rustup target add wasm32-wasip2
```

```sh
cargo install wasm-tools --locked
```

## Generate WIT Bindings

```sh
cargo component bindings
```

## Build the Component

```sh
cargo build --target wasm32-wasip2 --release
```

## Inspect Component

Inspect the components imports and exports.

```sh
wasm-tools component wit target/wasm32-wasip2/release/component.wasm
```

(Component has to be build first)

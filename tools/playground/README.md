# Izel Playground (WASM Browser REPL)

This directory contains the browser playground that runs the Izel frontend pipeline in WebAssembly.

## Components

- `wasm/`: Rust-to-WASM bridge module exposed to JavaScript.
- `index.html`, `main.js`, `styles.css`: browser REPL host UI.
- `pkg/`: generated wasm-bindgen browser package output (created by build scripts).

## Build

```bash
cd tools/playground
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --locked
npm run build:wasm
```

## Run Locally

```bash
cd tools/playground
npm run serve
```

Then open http://localhost:4173.

The Run action (or Cmd/Ctrl+Enter) tokenizes, parses, lowers, and type-checks the source and prints diagnostics.

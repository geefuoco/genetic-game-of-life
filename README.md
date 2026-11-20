# Genetic Game of Life

Conway's Game of Life where every Cell carries colour genes, so ancestry shows as colour.

**Live site: <https://geefuoco.github.io/genetic-game-of-life/>**

The simulation is a headless Rust core (`crates/core`) that also runs from a CLI (`crates/cli`). A WebAssembly layer (`crates/wasm`) and a Svelte front end (`web`) render it in the browser. The domain language is defined in [GLOSSARY.md](GLOSSARY.md) and the key decisions are in [docs/adr](docs/adr).

## Developing

Rust:

```sh
cargo test --workspace
```

Web app, from `web/` (needs the `wasm32-unknown-unknown` target and `wasm-bindgen`):

```sh
npm ci
npm run wasm
npm run dev
```

## Deployment

Every merge to `main` builds the web app and deploys it to GitHub Pages. The build in `web/dist` is a self-contained static bundle with relative asset paths, so it also works when copied under any path on another site or embedded in an iframe.

## Licence

[MIT](LICENSE)

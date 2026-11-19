# Headless Rust core with a Svelte web renderer

The simulation lives in a pure Rust core crate with no browser or rendering dependencies, so it can run headless (CLI, tests, benchmarks). A thin wasm-bindgen crate exposes it to a Svelte + TypeScript frontend that only renders and handles input. We chose this over an all-Rust UI (egui/eframe) because the web is the primary target and DOM controls give accessibility, deep links, and easy embedding in the portfolio museum; the cost is that a native app is no longer free and would be a separate frontend over the same core.

# Workstreams API

API for workstreams, written in Rust and served via Cloudflare Workers.

## Linting

```bash
cargo check --all
cargo +nightly fmt -- --check
cargo +nightly clippy --all --all-features -- -D warnings
```
## Testing

For testing, we use `miniflare`, a tool developed by Cloudflare that enables to mock requests to our worker and perform e2e tests.

Due to the `wasm` target and the existence of async/await directives,  currently native testing in rust is no possible.

## License

Apache License

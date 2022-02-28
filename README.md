# Workstreams API

API for workstreams, written in Rust and served via Cloudflare Workers.

## Testing and Linting

```bash
cargo check --all
cargo test --all --all-features
cargo +nightly fmt -- --check
cargo +nightly clippy --all --all-features -- -D warnings
```

## License

Apache License

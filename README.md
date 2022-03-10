# Workstreams API

API for workstreams, written in Rust and served via Cloudflare Workers.

The API's documentation is hosted in [GitHub Pages](https://radicle-dev.github.io/workstreams-api/workstreams_api/).


## Linting

```bash
cargo check --all
cargo +nightly fmt -- --check
cargo +nightly clippy --all --all-features -- -D warnings
```

## CI

- The documentation is built and pushed with every change. It's hosted automatically by GitHub pages so it's always up to date.
- The worker is automatically published with every commit to master
-
## License

Apache License

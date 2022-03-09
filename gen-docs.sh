#!/usr/bin/env bash

rm -rf ./target/doc
cargo doc --document-private-items --no-deps --release
rm -rf ./docs
echo "<meta http-equiv=\"refresh\" content=\"0; url=workstreams_api\">" > target/doc/index.html
cp -r target/doc ./docs

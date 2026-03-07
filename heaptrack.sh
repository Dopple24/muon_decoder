#!/bin/bash
rm heaptrack.particle_decoder*
RUSTFLAGS="-C force-frame-pointers=yes -C debuginfo=2 -C opt-level=2 -C codegen-units=1 -C lto=no -C dwarf-version=4 -C split-debuginfo=off -C link-dead-code -C llvm-args=--inline-threshold=0" \
cargo heaptrack
zstd -dc < $(pwd)/heaptrack.particle_decoder.*.raw.zst | /usr/lib/heaptrack/libexec/heaptrack_interpret | zstd -c > "$(pwd)/heaptrack.zst"

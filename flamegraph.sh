RUSTFLAGS="-C force-frame-pointers=yes -C debuginfo=2 -C opt-level=2 -C codegen-units=1 -C lto=no -C dwarf-version=4 -C split-debuginfo=off -C link-dead-code -C llvm-args=--inline-threshold=0" \
cargo flamegraph

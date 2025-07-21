# Set optimization flags
export RUSTFLAGS="-C embed-bitcode=yes -C codegen-units=1 -C opt-level=3 --cfg web_sys_unstable_apis --cfg getrandom_backend=\"wasm_js\""

# Run wasm pack tool to build JS wrapper files and copy wasm to pkg directory.
mkdir -p pkg
cargo fmt
wasm-pack build --out-dir pkg --release --target web --no-typescript
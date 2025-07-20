cargo build --release
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir ./out ./target/wasm32-unknown-unknown/release/wgpu-rs-learn.wasm
#!/bin/bash
set -e

echo "🔨 Building WASM..."
cargo build --target wasm32-unknown-unknown --release -p web_app

echo "📦 Running wasm-bindgen..."
wasm-bindgen --target web --out-dir web_app/pkg --no-typescript target/wasm32-unknown-unknown/release/web_app.wasm

echo "✅ Build complete! Files are in web_app/pkg/"
echo ""
echo "To run the app:"
echo "1. Start the simulator: cargo run -p sim -- --realtime"
echo "2. Start the WebSocket server: cargo run --manifest-path web_app/Cargo_server.toml"
echo "3. Serve the web app: cd web_app && python3 -m http.server 8000"
echo "4. Open http://localhost:8000 in your browser"


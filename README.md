cargo build --target wasm32-unknown-unknown --release
cargo test --test lookup_test -- --nocapture

# Optional: test-only env vars
# Create .env.test with values like:
# MAL_CLIENT_ID=your_client_id_here

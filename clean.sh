rustup run nightly cargo fmt -- --check
cargo clippy --fix --allow-dirty --allow-staged --workspace --all-targets --all-features
rustup run nightly cargo clippy --fix --allow-dirty --allow-staged --workspace --all-features --all-targets

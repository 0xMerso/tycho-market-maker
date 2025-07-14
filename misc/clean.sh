# 1. Auto-format all files
cargo fmt

# 2. Auto-apply all Clippy suggestions in place
# (allows fixing on a dirty or staged workspace)
cargo clippy \
    --fix \
    --allow-dirty \
    --allow-staged \
    --workspace \
    --all-targets \
    --all-features

# 3. Re-format after Clippy fix (in case Clippy changed formatting)
cargo fmt

# Common commands for CI

set shell := ["sh", "-c"]
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Run all CI targets
ci: check fmt clippy test
    @echo 'All checks passed!'

# Ensure all checks pass
check:
    cargo check --workspace --all-targets --all-features
    cargo check --workspace --all-targets --no-default-features

# Ensure all code is properly formatted
fmt:
    cargo fmt --all

# Run Clippy with all warnings enabled
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all
    cargo clippy --workspace --all-targets --no-default-features -- -D warnings -W clippy::all

# Run documentation tests
doctest:
    cargo test --workspace --doc

# Run doc and normal tests with the default runner (not nextest)
test: 
    cargo test --workspace --all-targets --all-features
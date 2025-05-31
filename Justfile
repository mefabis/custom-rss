# List available commands
help:
    just --list

# Build the project without running
build profile:
    cargo build --profile {{ profile }}

# Build and run the project
run profile:
    cargo run --profile {{ profile }}

# Build and test the project
test profile:
    cargo test --profile {{ profile }}

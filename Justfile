all: lint test build image

lint:
    cargo clippy -- -D warnings

test:
    cargo test

build:
    cargo build --release

image:
    podman build -t vulma .

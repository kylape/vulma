all: lint test build image

lint:
    cargo clippy

test:
    cargo test

build:
    cargo build --release

image:
    podman build -t vulma .

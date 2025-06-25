all: lint test build image

lint:
    cargo clippy -- -D warnings

test:
    cargo test

build:
    cargo build --release

image:
    podman build -t quay.io/mmoltras/vulma .

run *args: image
    podman run --rm -it --name vulma \
        --network host \
        -v /:/host:ro \
        quay.io/mmoltras/vulma {{args}}

mock-server:
    cargo run --release --bin mock-server

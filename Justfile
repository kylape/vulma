all: lint test build image

lint:
    cargo clippy -- -D warnings

test:
    cargo test

build:
    cargo build --release

image:
    podman build -t quay.io/mmoltras/vulma .

run +args: image
    podman run --rm -it --name vulma \
        --network host \
        -v /var/lib/rpm:/host/var/lib/rpm:ro \
        quay.io/mmoltras/vulma {{args}}

mock-server:
    uv run mock-server/server.py

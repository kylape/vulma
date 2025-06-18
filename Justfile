all: lint test build image

lint:
    cargo clippy -- -D warnings

test:
    cargo test

build:
    cargo build --release

image:
    podman build -t quay.io/mmoltras/vulma .

run: image
    podman run --rm -it --name vulma \
        -v /var/lib/rpm:/host/var/lib/rpm:ro \
        quay.io/mmoltras/vulma

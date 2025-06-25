FROM registry.access.redhat.com/ubi8/ubi:8.10 as builder

RUN dnf install -y \
        cargo-1.84.1 \
        rust-1.84.1  \
        protobuf \
        protobuf-compiler && \
    mkdir /app

WORKDIR /app

COPY . .

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin vulma && \
    cp target/release/vulma vulma

FROM registry.access.redhat.com/ubi8/ubi-minimal:8.10

ENV VULMA_RPMDB=/host/var/lib/rpm
ENV VULMA_HOST_MOUNT=/host

COPY --from=builder /app/vulma /usr/local/bin

ENTRYPOINT ["vulma"]

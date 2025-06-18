FROM registry.access.redhat.com/ubi8/ubi:8.10 as builder

RUN dnf install -y \
        cargo-1.84.1 \
        rust-1.84.1 && \
    mkdir /app

WORKDIR /app

COPY . .

RUN cargo build --release

FROM registry.access.redhat.com/ubi8/ubi-minimal:8.10

ENV VULMA_RPMDB=/host/var/lib/rpm
COPY --from=builder /app/target/release/vulma /usr/local/bin

ENTRYPOINT ["vulma"]

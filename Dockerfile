FROM rust:1.40 as builder
WORKDIR /boost-guard
COPY . .
RUN rustup override set nightly; \
    cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/boost-guard /usr/local/bin/boost-guard
CMD boost-guard

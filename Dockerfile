FROM rust:1.40 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN rustup override set nightly; \
    cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/myapp /usr/local/bin/myapp
ENV ROCKET_PORT 8080
CMD myapp

FROM rust:1.74.1 as builder

RUN USER=root cargo new --bin boost-guard

WORKDIR /boost-guard

COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/boost_guard*

RUN cargo build --release

FROM debian:buster-slim as runtime

WORKDIR /bin

# Copy from builder and rename to 'server'
COPY --from=builder /boost-guard/target/release/boost-guard ./server

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Etc/UTC \
    USER=appuser

RUN groupadd ${USER} \
    && useradd -g ${USER} ${USER} && \
    chown -R ${USER}:${USER} /bin

USER ${USER}

ENTRYPOINT ["./server"]
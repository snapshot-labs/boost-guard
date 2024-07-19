FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin boost-guard

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl-dev libssl3
WORKDIR /app
COPY --from=builder /app/target/release/boost-guard /usr/local/bin

ENV SEPOLIA_SUBGRAPH_URL "https://subgrapher.snapshot.org/subgraph/arbitrum/6T64qrPe7S46zhArSoBF8CAmc5cG3PyKa92Nt4Jhymcy"
ENV MAINNET_SUBGRAPH_URL "https://subgrapher.snapshot.org/subgraph/arbitrum/A6EEuSAB7mFrWvLBnL1HZXwfiGfqFYnFJjc14REtMNkd"
ENV POLYGON_SUBGRAPH_URL "https://subgrapher.snapshot.org/subgraph/arbitrum/CkNpf5gY7XPCinJWP1nh8K7u6faXwDjchGGV4P9rgJ7"
ENV BASE_SUBGRAPH_URL "https://subgrapher.snapshot.org/subgraph/arbitrum/52uVpyUHkkMFieRk1khbdshUw26CNHWAEuqLojZzcyjd"
ENV BOOST_NAME "boost"
ENV BOOST_VERSION "0.1.0"
ENV VERIFYING_CONTRACT "0x8E8913197114c911F13cfBfCBBD138C1DC74B964"
ENV SLOT_URL "https://beaconcha.in/api/v1/slot/"
ENV EPOCH_URL "https://beaconcha.in/api/v1/epoch/"

ENTRYPOINT ["/usr/local/bin/boost-guard"]
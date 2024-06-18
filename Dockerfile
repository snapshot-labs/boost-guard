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

ENV SEPOLIA_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost-sepolia/version/latest"
ENV MAINNET_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost/version/latest"
ENV POLYGON_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost-polygon/version/latest"
ENV BASE_SUBGRAPH_URL "https://api.studio.thegraph.com/query/23545/boost-base/version/latest"
ENV BOOST_NAME "boost"
ENV BOOST_VERSION "0.1.0"
ENV VERIFYING_CONTRACT "0x8E8913197114c911F13cfBfCBBD138C1DC74B964"
ENV SLOT_URL "https://beaconcha.in/api/v1/slot/"
ENV EPOCH_URL "https://beaconcha.in/api/v1/epoch/"

ENTRYPOINT ["/usr/local/bin/boost-guard"]
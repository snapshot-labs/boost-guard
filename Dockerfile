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

# NOTE: This will be removed, it's a random private key taken from the internet.
ENV PRIVATE_KEY "0xafdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890"

ENV HUB_URL "https://testnet.hub.snapshot.org/graphql"
ENV SUBGRAPH_URL "https://api.thegraph.com/subgraphs/name/snapshot-labs/boost-sepolia"
ENV BOOST_NAME "boost"
ENV BOOST_VERSION "1"
ENV VERIFYING_CONTRACT "0x3a18420C0646CC8e6D46E43d792335AeCB657fd0"

ENTRYPOINT ["/usr/local/bin/boost-guard"]
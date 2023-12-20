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

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install ca-certificates libssl-dev libssl3
WORKDIR /app
COPY --from=builder /app/target/release/boost-guard /usr/local/bin
# todo: remove
ENV PRIVATE_KEY 0xafdfd9c3d2095ef696594f6cedcae59e72dcd697e2a7521b1578140422a4f890
ENTRYPOINT ["/usr/local/bin/boost-guard"]

# RUN apt-get update \
#     && apt-get install -y ca-certificates tzdata \
#     && rm -rf /var/lib/apt/lists/*

# ENV TZ=Etc/UTC \
#     USER=appuser

# RUN groupadd ${USER} \
#     && useradd -g ${USER} ${USER} && \
#     chown -R ${USER}:${USER} /bin

# USER ${USER}
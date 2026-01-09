FROM public.ecr.aws/docker/library/rust:1.92.0-bookworm AS base
WORKDIR /app

# Install cargo chef
RUN cargo install sccache --version ^0.9
RUN cargo install cargo-chef --version ^0.1

ENV CARGO_HOME=/usr/local/cargo
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/sccache

FROM base AS planner

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
  --mount=type=cache,target=/usr/local/cargo/git \
  --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
  cargo chef prepare  --recipe-path recipe.json

FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
        cargo chef cook --release --recipe-path recipe.json

# Build application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release --bin world-id-relay


### Runtime stage
## cc variant because we need libgcc and others
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
ENV RUST_LOG="info"
COPY --from=builder -from=builder --chown=0:10001 --chmod=454 /app/target/release/world-id-relay /usr/local/bin/
ENTRYPOINT [ "/usr/local/bin/world-id-relay" ]

FROM debian:12 AS chef

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    curl build-essential \
    libssl-dev texinfo \
    libcap2-bin pkg-config

# Install rustup
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y

COPY ./rust-toolchain.toml ./rust-toolchain.toml

# Set environment variables
ENV PATH="/root/.cargo/bin:${PATH}"
ENV RUSTUP_HOME="/root/.rustup"
ENV CARGO_HOME="/root/.cargo"

# Install the toolchain
RUN rustup component add cargo

# Install cargo chef
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release


### Runtime stage
## cc variant because we need libgcc and others
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app
COPY --from=builder --chown=0:10001 --chmod=454 /app/target/release/world-id-relay /bin/app
ENTRYPOINT [ "/bin/app" ]

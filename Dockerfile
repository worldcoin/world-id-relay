### Base stage
FROM debian:12 AS base

WORKDIR /src

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    curl build-essential \
    libssl-dev texinfo \
    libcap2-bin pkg-config

# TODO: Use a specific version of rustup
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

### Recipe cooking stage
FROM base AS build-env-base
WORKDIR /src

# Copy everything
COPY . .

# Prepare the recipe
RUN cargo chef prepare --recipe-path recipe.json

### Build stage
FROM base AS build-env
WORKDIR /src

# Copy recipe
COPY --from=build-env-base /src/recipe.json ./recipe.json

COPY . .

# Build the dependencies
RUN cargo chef cook --release --recipe-path ./recipe.json

# Build the binary
RUN cargo build --release

### Runtime stage
## cc variant because we need libgcc and others
FROM gcr.io/distroless/cc-debian12:nonroot

# Copy the binary
COPY --from=build-env --chown=0:10001 --chmod=454 /src/target/release/world-id-relay /bin/app

ENTRYPOINT [ "/bin/app" ]

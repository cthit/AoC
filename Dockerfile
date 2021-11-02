FROM rust:1.55 AS build

WORKDIR /usr/src

RUN rustup default nightly

# Allow caching of dependencies
RUN cargo new --bin --vcs none digit-aoc
WORKDIR /usr/src/digit-aoc
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

# Build and install the binary
COPY src/ ./src/
# Update timestamp so that cargo builds again
RUN touch src/main.rs
RUN cargo install --path .

FROM debian:buster-slim AS run
# OpenSSL is needed
RUN apt-get update && apt-get install openssl -y
# Get the binary
COPY --from=build /usr/local/cargo/bin/digit-aoc ./digit-aoc
# Run the binary
CMD ["./digit-aoc"]

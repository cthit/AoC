FROM rust:1.55 AS build

WORKDIR /usr/src

RUN apt-get update && apt-get install libpq-dev ca-certificates -y && update-ca-certificates
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
RUN cargo build --release

FROM debian:buster-slim AS run
# OpenSSL is needed
RUN apt-get update && apt-get install openssl libpq-dev -y
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# Get statics and templates
COPY static/ ./static
COPY templates/ ./templates
# Get the binary
COPY --from=build /usr/src/digit-aoc/target/release/digit-aoc ./digit-aoc
# Run the binary
CMD ["./digit-aoc"]

FROM rust:alpine AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests
RUN cargo build --release

FROM alpine:latest
COPY --from=build /app/target/release/portpeek /usr/local/bin/portpeek
ENTRYPOINT ["portpeek"]

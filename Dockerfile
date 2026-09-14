FROM rust:1.97-alpine AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

FROM alpine:3.22
COPY --from=build /app/target/release/portpeek /usr/local/bin/portpeek
ENTRYPOINT ["portpeek"]

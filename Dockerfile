FROM rust:1-slim-buster AS build

RUN cargo new --bin app

WORKDIR /app

RUN cargo new --bin load_balancer_http
RUN cargo new --bin load_balancer_tcp
RUN cargo new --bin load_test

COPY Cargo.toml /app/
COPY Cargo.lock /app/
RUN cargo build --release

COPY src /app/src
RUN touch /app/src/main.rs
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /app/target/release/rinha_de_backend_2024_q1_rust /app/rinha

CMD "/app/rinha"

FROM rust:1-slim-buster AS build

RUN cargo new --bin app

WORKDIR /app

RUN cargo new --bin load_balancer_tcp

COPY load_balancer_tcp/Cargo.toml /app/
COPY Cargo.lock /app/
RUN cargo build --release

COPY load_balancer_tcp/src /app/src
RUN touch /app/src/main.rs
RUN cargo build --release

FROM debian:buster-slim

COPY --from=build /app/target/release/load_balancer_tcp /app/rinha

CMD "/app/rinha"

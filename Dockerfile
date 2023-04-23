# FROM rust:1.69.0 as builder
# ENV SQLX_OFFLINE true
# WORKDIR /app
# RUN apt update && apt install lld clang -y
# COPY . .
# RUN cargo build --release

# FROM debian:bullseye-slim AS runtime
# WORKDIR /app
# RUN apt-get update -y \
#     && apt-get install -y --no-install-recommends openssl ca-certificates \
#     && apt-get autoremove -y \
#     && apt-get clean -y \
#     && rm -rf /var/lib/apt/lists/*
# COPY config config
# COPY --from=builder /app/target/release/zero2prod zero2prod
# ENV APP_ENVIRONMENT production
# ENTRYPOINT [ "./zero2prod" ]

FROM lukemathwalker/cargo-chef:latest-rust-1.63.0 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin zero2prod

FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY config config
COPY --from=builder /app/target/release/zero2prod zero2prod
ENV APP_ENVIRONMENT production
ENTRYPOINT [ "./zero2prod" ]

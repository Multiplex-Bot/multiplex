FROM rustlang/rust:nightly-bookworm AS builder
WORKDIR /multiplex
COPY . .
RUN cargo install --path .

FROM debian:bookworm
RUN apt-get update && apt-get install -y openssl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/multiplex /usr/local/bin/multiplex
CMD ["multiplex"]
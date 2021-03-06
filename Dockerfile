FROM rust:1.50-slim-buster AS builder

WORKDIR /app
COPY src src/
COPY migrations migrations/
COPY Cargo.toml Cargo.lock build.rs ./
RUN cargo test
RUN cargo build --release



FROM debian:buster-slim

RUN apt-get update && apt-get install -y ca-certificates

RUN useradd -m -r links
WORKDIR /home/links

COPY --from=builder --chown=root:root /app/target/release/links /usr/bin/

USER links
ENTRYPOINT ["links"]
CMD ["run"]
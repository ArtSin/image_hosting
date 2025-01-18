FROM rust:1.84-bookworm AS builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz \
    && tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz \
    && mv cargo-binstall /usr/local/cargo/bin

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, where everything will eventually live in
RUN mkdir -p /app
WORKDIR /app
COPY . .

# Build the app
ENV SQLX_OFFLINE="true"
RUN cargo leptos build --release -vv \
    && cargo build --release --bin worker \
    && mkdir -p /app/artifacts \
    && mv target/release/image-hosting artifacts/ \
    && mv target/release/worker artifacts/ \
    && mv target/site artifacts/ \
    && cargo clean

FROM debian:bookworm-slim AS runtime
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user with an explicit UID and add permission to access the /app folder
RUN adduser -u 5678 --disabled-password --gecos "" appuser && mkdir -p /app && chown -R appuser /app
USER appuser

RUN mkdir -p /app/storage/images && mkdir -p /app/storage/thumbnails

FROM runtime AS image-hosting
WORKDIR /app

# Copy the server binary to the /app directory
COPY --from=builder /app/artifacts/image-hosting /app/

# /target/site contains JS/WASM/CSS, etc.
COPY --from=builder /app/artifacts/site /app/site

# Set required env variables
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# Run the server
CMD ["./image-hosting"]

FROM runtime AS worker
WORKDIR /app

COPY --from=builder /app/artifacts/worker /app/

COPY --from=builder /app/models /app/models

CMD ["./worker"]

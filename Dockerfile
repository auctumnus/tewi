# Build stage
FROM rust:1.88-slim AS builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    git \
    unzip \
    && rm -rf /var/lib/apt/lists/*

# Install Bun
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"
ENV DATABASE_URL="postgres://user:password@localhost:5432/tewi"

WORKDIR /app

# Copy monorepo files
COPY ./package.json ./bun.lock ./

# Copy frontend files and build
COPY frontend/package.json frontend/bun.lock ./frontend/
RUN cd frontend && bun install --frozen-lockfile

COPY frontend ./frontend
RUN cd frontend && bun run build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY templates ./templates

# Copy asset files
COPY assets ./assets

RUN SQLX_OFFLINE=true cargo build --release

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/tewi ./
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/frontend/dist ./frontend/dist
COPY --from=builder /app/assets ./assets

EXPOSE 3000

CMD ["./tewi"]
# ══════════════════════════════════════════════════════════════════════════════
#  VulnSamurai — single-container build
#  Services (managed by supervisord):
#    mongodb  → localhost:27017
#    api      → localhost:8000  (Rust/axum)
#    frontend → localhost:3000  (Node.js)
# ══════════════════════════════════════════════════════════════════════════════

# ── Stage 1: Build the Rust API ───────────────────────────────────────────────
# Rust 1.85+ required for edition2024 deps pulled in by chrono/time
FROM rust:1.85-slim-bookworm AS rust_builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependency compile layer.
# Cargo.lock is auto-generated on first build — only Cargo.toml is required.
COPY backend/Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Full build
COPY backend/src ./src
RUN touch src/main.rs && cargo build --release

# ── Stage 2: Runtime image ────────────────────────────────────────────────────
FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

# ── System packages + scan tools ──────────────────────────────────────────────
RUN apt-get update && apt-get install -y \
    curl wget gnupg unzip \
    supervisor \
    ca-certificates \
    libssl1.1 \
    nikto whatweb gobuster sqlmap wapiti \
    && rm -rf /var/lib/apt/lists/*

# ── Node.js 18 ────────────────────────────────────────────────────────────────
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs && \
    rm -rf /var/lib/apt/lists/*

# ── MongoDB 4.4 (no AVX required) ────────────────────────────────────────────
RUN wget -qO - https://www.mongodb.org/static/pgp/server-4.4.asc | apt-key add - && \
    echo "deb [ arch=amd64,arm64 ] https://repo.mongodb.org/apt/ubuntu focal/mongodb-org/4.4 multiverse" \
        > /etc/apt/sources.list.d/mongodb-org-4.4.list && \
    apt-get update && \
    apt-get install -y mongodb-org && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# ── Nuclei (latest) ───────────────────────────────────────────────────────────
RUN NUCLEI_VER=$(curl -s https://api.github.com/repos/projectdiscovery/nuclei/releases/latest \
        | grep '"tag_name"' | head -1 | sed 's/.*"v\([^"]*\)".*/\1/') && \
    curl -sL "https://github.com/projectdiscovery/nuclei/releases/download/v${NUCLEI_VER}/nuclei_${NUCLEI_VER}_linux_amd64.zip" \
        -o /tmp/nuclei.zip && \
    unzip /tmp/nuclei.zip -d /usr/local/bin/ && \
    chmod +x /usr/local/bin/nuclei && \
    rm /tmp/nuclei.zip

# ── Directories ───────────────────────────────────────────────────────────────
RUN mkdir -p /data/db /data/log /app/frontend/static

# ── Rust API binary (from builder stage) ─────────────────────────────────────
COPY --from=rust_builder /build/target/release/api /app/api
RUN chmod +x /app/api

# ── Frontend (Node.js) ────────────────────────────────────────────────────────
COPY frontend/ /app/frontend/
RUN cd /app/frontend && npm install --omit=dev

# ── Config files ──────────────────────────────────────────────────────────────
COPY supervisord.conf /etc/supervisor/conf.d/vulnsamurai.conf
COPY entrypoint.sh    /entrypoint.sh
RUN chmod +x /entrypoint.sh

EXPOSE 3000

ENTRYPOINT ["/entrypoint.sh"]

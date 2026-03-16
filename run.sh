#!/bin/bash
# VulnSamurai — Linux / macOS launcher
set -e

echo ""
echo "  ⚔  VulnSamurai"
echo "======================================"

# Auto-generate JWT secret into .env (only if still placeholder)
if grep -q "changeme_run_openssl_rand_hex_64_and_paste_here" .env 2>/dev/null; then
    JWT=$(openssl rand -hex 64)
    sed -i "s|changeme_run_openssl_rand_hex_64_and_paste_here|${JWT}|g" .env
    echo "[0/2] JWT secret generated."
fi

echo "[1/2] Building image (first run ~20 min — Rust compile + scan tools)..."
docker build -t vulnsamurai .

echo "[2/2] Starting container..."
docker rm -f vulnsamurai 2>/dev/null || true
docker run -d \
  --name vulnsamurai \
  --env-file .env \
  -p 3000:3000 \
  -v vulnsamurai_data:/data \
  vulnsamurai

echo ""
echo "======================================"
echo "  Ready at http://localhost:3000"
echo "  Logs:  docker logs -f vulnsamurai"
echo ""
echo "  Per-service logs:"
echo "  docker exec vulnsamurai tail -f /data/log/backend.log"
echo "  docker exec vulnsamurai tail -f /data/log/frontend.log"
echo "  docker exec vulnsamurai tail -f /data/log/mongod.log"
echo "======================================"

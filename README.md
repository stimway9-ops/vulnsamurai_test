# VulnSamurai — Rust Backend

Replaces the Python/Flask backend with an async Rust API using **axum** + **MongoDB**.

## Stack

| Layer      | Crate / Image                  |
|------------|-------------------------------|
| HTTP       | axum 0.7                      |
| Async RT   | tokio                         |
| Database   | mongodb 2 (official async)    |
| Auth       | jsonwebtoken 9 + bcrypt 0.15  |
| Validation | validator 0.18                |
| Proxy      | nginx 1.25-alpine             |
| DB         | mongo 7                       |

## Project layout

```
.
├── backend/
│   ├── Cargo.toml
│   ├── Dockerfile          ← multi-stage build (builder + slim runtime)
│   └── src/
│       ├── main.rs
│       ├── db/mod.rs       ← AppState, MongoDB connection
│       ├── middleware/
│       │   └── auth.rs     ← JWT extractor (axum FromRequestParts)
│       ├── models/
│       │   ├── user.rs     ← User, Claims, DTOs
│       │   ├── scan.rs     ← ScanJob, ScanEngine, DTOs
│       │   └── vuln.rs     ← VulnEntry, Severity, DTOs
│       └── handlers/
│           ├── mod.rs      ← shared ApiError
│           ├── auth.rs     ← POST /api/auth/register|login
│           ├── scans.rs    ← CRUD /api/scans
│           ├── vulns.rs    ← CRUD /api/vulns
│           └── health.rs   ← GET /health
├── nginx/nginx.conf        ← reverse proxy: /api/* → api:8000, /* → frontend:3000
├── mongo-init/01_indexes.js← runs once on first DB start
├── docker-compose.yml
├── .env                    ← gitignored, copy from .env.example
└── .env.example
```

## Quick start

```bash
# 1. Copy and edit env
cp .env.example .env

# 2. Build and start all services
docker compose up --build

# 3. API is available via nginx at http://localhost/api/
#    Or directly at http://localhost:8000 (not exposed by default)
```

## API reference

### Auth

| Method | Path                    | Auth | Description       |
|--------|-------------------------|------|-------------------|
| POST   | /api/auth/register      | —    | Register new user |
| POST   | /api/auth/login         | —    | Login, get JWT    |

**Register body:**
```json
{ "username": "neo", "email": "neo@matrix.io", "password": "redpill123" }
```

**Login body:**
```json
{ "email": "neo@matrix.io", "password": "redpill123" }
```

Both return:
```json
{ "token": "<jwt>", "user": { "id": "...", "username": "...", "email": "...", "role": "analyst", "created_at": "..." } }
```

All protected routes require header:
```
Authorization: Bearer <token>
```

---

### Scans

| Method | Path                        | Description                          |
|--------|-----------------------------|--------------------------------------|
| POST   | /api/scans                  | Create scan job                      |
| GET    | /api/scans                  | List your scans (admin sees all)     |
| GET    | /api/scans/:id              | Get single scan                      |
| PUT    | /api/scans/:id/status       | Update status (queued/running/completed/failed) |
| POST   | /api/scans/:id/results      | Append engine result                 |

**Create scan body:**
```json
{
  "target": "https://example.com",
  "engines": ["nikto", "gobuster", "nuclei"]
}
```

**Append result body:**
```json
{
  "engine": "nikto",
  "output": "- Nikto v2.1.6\n+ Target IP: ...",
  "exit_code": 0
}
```

---

### Vulns

| Method | Path           | Description                       |
|--------|----------------|-----------------------------------|
| POST   | /api/vulns     | Store a discovered vulnerability  |
| GET    | /api/vulns     | List vulns (filterable)           |
| GET    | /api/vulns/:id | Get single vuln                   |
| DELETE | /api/vulns/:id | Delete a vuln                     |

**Query params for GET /api/vulns:**
- `severity` — critical / high / medium / low / info
- `scan_id` — filter by scan
- `page` — default 1
- `limit` — default 20, max 100

**Create vuln body:**
```json
{
  "scan_id": "<objectid>",
  "title": "SQL Injection in /login",
  "description": "Unsanitized input in the email field.",
  "severity": "critical",
  "cve_id": "CVE-2024-1234",
  "cvss_score": 9.8,
  "affected_url": "https://example.com/login",
  "evidence": "' OR 1=1 --  → 200 OK with all users",
  "tags": ["sqli", "auth"]
}
```

---

## Integrating with the existing frontend / scan runners

Your scan runner scripts (nikto, gobuster, etc.) should:

1. **POST /api/scans** to create a job → get back `scan.id`
2. **PUT /api/scans/:id/status** `{ "status": "running" }` when a tool starts
3. **POST /api/scans/:id/results** for each tool's output
4. **PUT /api/scans/:id/status** `{ "status": "completed" }` when all done

## Useful Docker commands

```bash
docker compose up --build -d   # start in background
docker compose logs -f api     # live API logs
docker compose logs -f mongo   # live DB logs
docker compose down            # stop (keeps mongo_data volume)
docker compose down -v         # stop + wipe DB
```

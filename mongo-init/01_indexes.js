// Runs once when the mongo container is first created.
// Creates indexes for the vulnsamurai DB.

const db = connect(`mongodb://localhost:27017/vulnsamurai`);

// ── users ────────────────────────────────────────────────────────────────────
db.users.createIndex({ email: 1 }, { unique: true });
db.users.createIndex({ username: 1 });

// ── scans ────────────────────────────────────────────────────────────────────
db.scans.createIndex({ owner_id: 1 });
db.scans.createIndex({ status: 1 });
db.scans.createIndex({ created_at: -1 });

// ── vulns ────────────────────────────────────────────────────────────────────
db.vulns.createIndex({ owner_id: 1 });
db.vulns.createIndex({ scan_id: 1 });
db.vulns.createIndex({ severity: 1 });
db.vulns.createIndex({ created_at: -1 });
// Compound: filter by owner + severity (common dashboard query)
db.vulns.createIndex({ owner_id: 1, severity: 1, created_at: -1 });

print("✔  Indexes created for vulnsamurai");

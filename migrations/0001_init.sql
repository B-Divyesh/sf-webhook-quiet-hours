PRAGMA foreign_keys = ON;

CREATE TABLE endpoints (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  token_hash TEXT NOT NULL,
  signing_secret_encrypted BLOB,
  created_at TEXT NOT NULL
);

CREATE TABLE fingerprints (
  fingerprint TEXT PRIMARY KEY,
  endpoint_id INTEGER NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  first_seen TEXT NOT NULL,
  last_seen TEXT NOT NULL,
  total_count INTEGER NOT NULL DEFAULT 1,
  pending_count INTEGER NOT NULL DEFAULT 1,
  severity TEXT NOT NULL DEFAULT 'normal' CHECK (severity IN ('normal', 'high', 'ignored')),
  target_minutes INTEGER NOT NULL DEFAULT 30,
  acknowledged_at TEXT,
  last_notified_at TEXT
);

CREATE INDEX idx_fingerprints_last_seen ON fingerprints(last_seen DESC);

CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  endpoint_id INTEGER NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
  fingerprint TEXT NOT NULL REFERENCES fingerprints(fingerprint) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  received_at TEXT NOT NULL,
  payload_encrypted BLOB NOT NULL,
  signature_valid INTEGER NOT NULL
);

CREATE INDEX idx_events_received_at ON events(received_at DESC);
CREATE INDEX idx_events_fingerprint ON events(fingerprint, received_at DESC);

CREATE TABLE settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  quiet_start TEXT NOT NULL DEFAULT '22:00',
  quiet_end TEXT NOT NULL DEFAULT '07:00',
  utc_offset_minutes INTEGER NOT NULL DEFAULT 0,
  digest_minutes INTEGER NOT NULL DEFAULT 60,
  retention_days INTEGER NOT NULL DEFAULT 7,
  notification_url_encrypted BLOB,
  escalation_url TEXT,
  last_delivery_error TEXT,
  updated_at TEXT NOT NULL
);

INSERT INTO settings (id, updated_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));


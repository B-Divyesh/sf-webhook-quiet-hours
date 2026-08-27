use std::{collections::BTreeSet, env, path::PathBuf, sync::Arc, time::Duration};

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Duration as ChronoDuration, Timelike, Utc};
use hmac::{Hmac, Mac};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor, GovernorLayer,
};
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing::{error, warn};

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub admin_token: String,
    pub encryption_key: [u8; 32],
    pub public_url: String,
    pub build_sha: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let production = env::var("APP_ENV").as_deref() == Ok("production");
        let admin_token = env::var("ADMIN_TOKEN").unwrap_or_else(|_| "local-dev-token".into());
        if production && admin_token == "local-dev-token" {
            return Err(AppError::Config(
                "ADMIN_TOKEN is required in production".into(),
            ));
        }
        let encryption_key = match env::var("DATA_ENCRYPTION_KEY") {
            Ok(raw) => {
                let decoded = BASE64
                    .decode(raw)
                    .map_err(|_| AppError::Config("DATA_ENCRYPTION_KEY must be base64".into()))?;
                decoded.try_into().map_err(|_| {
                    AppError::Config("DATA_ENCRYPTION_KEY must decode to exactly 32 bytes".into())
                })?
            }
            Err(_) if production => {
                return Err(AppError::Config(
                    "DATA_ENCRYPTION_KEY is required in production".into(),
                ))
            }
            Err(_) => Sha256::digest(b"webhook-quiet-hours-local-development-only").into(),
        };
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://data/quiet-hours.db?mode=rwc".into()),
            admin_token,
            encryption_key,
            public_url: env::var("PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into())
                .trim_end_matches('/')
                .into(),
            build_sha: env::var("BUILD_SHA").unwrap_or_else(|_| "development".into()),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    cipher: Arc<Aes256Gcm>,
    admin_hash: [u8; 32],
    public_url: String,
    build_sha: String,
    http: reqwest::Client,
}

impl AppState {
    pub async fn connect(config: &AppConfig) -> Result<Self, AppError> {
        if let Some(path) = config
            .database_url
            .strip_prefix("sqlite://")
            .and_then(|s| s.split('?').next())
        {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect(&config.database_url)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self {
            pool,
            cipher: Arc::new(
                Aes256Gcm::new_from_slice(&config.encryption_key).expect("32-byte key"),
            ),
            admin_hash: Sha256::digest(config.admin_token.as_bytes()).into(),
            public_url: config.public_url.clone(),
            build_sha: config.build_sha.clone(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(4))
                .build()?,
        })
    }

    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, AppError> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let mut out = nonce_bytes.to_vec();
        out.extend(
            self.cipher
                .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
                .map_err(|_| AppError::Crypto)?,
        );
        Ok(out)
    }

    fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, AppError> {
        if encrypted.len() < 13 {
            return Err(AppError::Crypto);
        }
        self.cipher
            .decrypt(Nonce::from_slice(&encrypted[..12]), &encrypted[12..])
            .map_err(|_| AppError::Crypto)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("migration error")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("network error")]
    Network(#[from] reqwest::Error),
    #[error("file error")]
    Io(#[from] std::io::Error),
    #[error("encryption error")]
    Crypto,
    #[error("not found")]
    NotFound,
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("unauthorized")]
    Unauthorized,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::Invalid(m) => (StatusCode::BAD_REQUEST, m.as_str()),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Use the ADMIN_TOKEN configured on this server.",
            ),
            Self::NotFound => (StatusCode::NOT_FOUND, "That record no longer exists."),
            Self::Config(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.as_str()),
            _ => {
                error!(error = %self, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "The server could not complete this request.",
                )
            }
        };
        (status, Json(json!({"error": message}))).into_response()
    }
}

pub fn build_app(state: AppState, dist: PathBuf) -> Router {
    // Bound accidental/hostile floods before parsing or encrypting request bodies. A generous
    // global allowance is predictable behind a self-hosted reverse proxy.
    let hook_limit = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(GlobalKeyExtractor)
            .per_millisecond(10)
            .burst_size(200)
            .finish()
            .expect("valid webhook rate limit"),
    );
    let hook = Router::new()
        .route("/hooks/:slug", post(receive_webhook))
        .layer(GovernorLayer { config: hook_limit });
    let api = Router::new()
        .route("/summary", get(summary))
        .route("/endpoints", get(list_endpoints).post(create_endpoint))
        .route("/endpoints/:id", delete(remove_endpoint))
        .route("/fingerprints", get(list_fingerprints))
        .route(
            "/fingerprints/:fingerprint",
            get(fingerprint_detail).patch(update_fingerprint),
        )
        .route("/fingerprints/:fingerprint/ack", post(ack_fingerprint))
        .route("/settings", get(get_settings).put(update_settings))
        .route("/digest/send", post(send_digest_now))
        .route("/export.csv", get(export_csv))
        .layer(middleware::from_fn_with_state(state.clone(), admin_auth));
    let static_service =
        ServeDir::new(&dist).not_found_service(ServeFile::new(dist.join("index.html")));
    Router::new()
        .route("/health", get(health))
        .merge(hook)
        .nest("/api", api)
        .fallback_service(static_service)
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self' https://api.sociobot.in https://pilot-api.sociobot.in; frame-ancestors 'none'; base-uri 'none'; form-action 'self' https://api.sociobot.in https://pilot-api.sociobot.in")))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn admin_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let supplied: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    if supplied != state.admin_hash {
        return Err(AppError::Unauthorized);
    }
    Ok(next.run(req).await)
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({"status":"ok", "build_sha": state.build_sha}))
}

#[derive(Serialize)]
struct Summary {
    endpoints: i64,
    fingerprints: i64,
    events_today: i64,
    pending: i64,
    compressed: i64,
    high_unacknowledged: i64,
}

async fn summary(State(state): State<AppState>) -> Result<Json<Summary>, AppError> {
    let endpoints: i64 = sqlx::query_scalar("SELECT count(*) FROM endpoints")
        .fetch_one(&state.pool)
        .await?;
    let fingerprints: i64 = sqlx::query_scalar("SELECT count(*) FROM fingerprints")
        .fetch_one(&state.pool)
        .await?;
    let events_today: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events WHERE received_at >= datetime('now', 'start of day')",
    )
    .fetch_one(&state.pool)
    .await?;
    let pending: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(pending_count),0) FROM fingerprints WHERE severity != 'ignored'",
    )
    .fetch_one(&state.pool)
    .await?;
    let total: i64 = sqlx::query_scalar("SELECT coalesce(sum(total_count),0) FROM fingerprints")
        .fetch_one(&state.pool)
        .await?;
    let high_unacknowledged: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM fingerprints WHERE severity='high' AND acknowledged_at IS NULL",
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(Summary {
        endpoints,
        fingerprints,
        events_today,
        pending,
        compressed: (total - fingerprints).max(0),
        high_unacknowledged,
    }))
}

#[derive(Serialize)]
struct EndpointView {
    id: i64,
    slug: String,
    name: String,
    signature_required: bool,
    created_at: String,
}

async fn list_endpoints(
    State(state): State<AppState>,
) -> Result<Json<Vec<EndpointView>>, AppError> {
    let rows = sqlx::query("SELECT id,slug,name,signing_secret_encrypted IS NOT NULL AS signed,created_at FROM endpoints ORDER BY created_at").fetch_all(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| EndpointView {
                id: r.get("id"),
                slug: r.get("slug"),
                name: r.get("name"),
                signature_required: r.get::<i64, _>("signed") != 0,
                created_at: r.get("created_at"),
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
struct CreateEndpoint {
    name: String,
    require_signature: Option<bool>,
}
#[derive(Serialize)]
struct CreatedEndpoint {
    id: i64,
    slug: String,
    name: String,
    hook_url: String,
    signing_secret: Option<String>,
}

async fn create_endpoint(
    State(state): State<AppState>,
    Json(input): Json<CreateEndpoint>,
) -> Result<(StatusCode, Json<CreatedEndpoint>), AppError> {
    let name = input.name.trim();
    if !(2..=60).contains(&name.len()) {
        return Err(AppError::Invalid(
            "Alias name must be 2–60 characters.".into(),
        ));
    }
    let slug_base: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(5)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    let slug = format!(
        "{}-{}",
        if slug_base.is_empty() {
            "endpoint"
        } else {
            &slug_base
        },
        suffix
    );
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let signing_secret = input.require_signature.unwrap_or(true).then(|| {
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect::<String>()
    });
    let encrypted = signing_secret
        .as_ref()
        .map(|s| state.encrypt(s.as_bytes()))
        .transpose()?;
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("INSERT INTO endpoints(slug,name,token_hash,signing_secret_encrypted,created_at) VALUES(?,?,?,?,?)")
        .bind(&slug).bind(name).bind(token_hash).bind(encrypted).bind(now).execute(&state.pool).await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedEndpoint {
            id: result.last_insert_rowid(),
            slug: slug.clone(),
            name: name.into(),
            hook_url: format!("{}/hooks/{}?key={}", state.public_url, slug, token),
            signing_secret,
        }),
    ))
}

async fn remove_endpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM endpoints WHERE id=?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct HookQuery {
    key: Option<String>,
}

async fn receive_webhook(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<HookQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    if body.len() > 262_144 {
        return Err(AppError::Invalid(
            "Payload exceeds the 256 KB limit.".into(),
        ));
    }
    let row =
        sqlx::query("SELECT id,token_hash,signing_secret_encrypted FROM endpoints WHERE slug=?")
            .bind(&slug)
            .fetch_optional(&state.pool)
            .await?
            .ok_or(AppError::NotFound)?;
    let expected: String = row.get("token_hash");
    let supplied = hex::encode(Sha256::digest(query.key.unwrap_or_default().as_bytes()));
    if supplied != expected {
        return Err(AppError::Unauthorized);
    }
    let encrypted_secret: Option<Vec<u8>> = row.get("signing_secret_encrypted");
    if let Some(secret_data) = encrypted_secret {
        let secret = state.decrypt(&secret_data)?;
        let signature = headers
            .get("x-hub-signature-256")
            .or_else(|| headers.get("x-webhook-signature"))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .strip_prefix("sha256=")
            .unwrap_or_else(|| {
                headers
                    .get("x-webhook-signature")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
            });
        let signature_bytes = hex::decode(signature).map_err(|_| AppError::Unauthorized)?;
        let mut mac =
            <Hmac<Sha256> as Mac>::new_from_slice(&secret).map_err(|_| AppError::Crypto)?;
        mac.update(&body);
        mac.verify_slice(&signature_bytes)
            .map_err(|_| AppError::Unauthorized)?;
    }
    let value: Value = serde_json::from_slice(&body)
        .unwrap_or_else(|_| json!({"raw": String::from_utf8_lossy(&body)}));
    let event_type = event_type(&value, &headers);
    let fingerprint = make_fingerprint(row.get("id"), &event_type, &value);
    let now = Utc::now().to_rfc3339();
    let payload = state.encrypt(&body)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO fingerprints(fingerprint,endpoint_id,event_type,first_seen,last_seen,total_count,pending_count) VALUES(?,?,?,?,?,1,1) ON CONFLICT(fingerprint) DO UPDATE SET last_seen=excluded.last_seen,total_count=total_count+1,pending_count=pending_count+1,acknowledged_at=CASE WHEN severity='high' THEN NULL ELSE acknowledged_at END")
        .bind(&fingerprint).bind(row.get::<i64,_>("id")).bind(&event_type).bind(&now).bind(&now).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO events(endpoint_id,fingerprint,event_type,received_at,payload_encrypted,signature_valid) VALUES(?,?,?,?,?,1)")
        .bind(row.get::<i64,_>("id")).bind(&fingerprint).bind(&event_type).bind(&now).bind(payload).execute(&mut *tx).await?;
    let count: i64 = sqlx::query_scalar("SELECT total_count FROM fingerprints WHERE fingerprint=?")
        .bind(&fingerprint)
        .fetch_one(&mut *tx)
        .await?;
    let severity: String =
        sqlx::query_scalar("SELECT severity FROM fingerprints WHERE fingerprint=?")
            .bind(&fingerprint)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    let signal = if severity == "high" {
        match notify_high(&state, &fingerprint, &event_type, count).await {
            Ok(true) => "escalated",
            Ok(false) => "recorded",
            Err(e) => {
                warn!(error=%e,"high-severity notification failed");
                "delivery_failed"
            }
        }
    } else if severity == "ignored" {
        "ignored"
    } else {
        "digest"
    };
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"accepted":true,"fingerprint":fingerprint,"count":count,"signal":signal})),
    ))
}

fn event_type(value: &Value, headers: &HeaderMap) -> String {
    headers
        .get("x-github-event")
        .or_else(|| headers.get("x-event-type"))
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| {
            ["type", "event", "action", "name"]
                .iter()
                .find_map(|k| value.get(k).and_then(Value::as_str).map(str::to_string))
        })
        .unwrap_or_else(|| "unclassified".into())
        .chars()
        .take(80)
        .collect()
}

fn make_fingerprint(endpoint_id: i64, event_type: &str, value: &Value) -> String {
    let mut keys = BTreeSet::new();
    if let Value::Object(map) = value {
        keys.extend(map.keys().map(String::as_str));
    }
    let status = value
        .get("status")
        .or_else(|| value.get("error"))
        .or_else(|| value.get("code"))
        .map(|v| v.to_string())
        .unwrap_or_default();
    let canonical = format!(
        "{}|{}|{}|{}",
        endpoint_id,
        event_type,
        keys.into_iter().collect::<Vec<_>>().join(","),
        status.chars().take(120).collect::<String>()
    );
    hex::encode(&Sha256::digest(canonical.as_bytes())[..10])
}

#[derive(Serialize)]
struct FingerprintView {
    fingerprint: String,
    endpoint_id: i64,
    endpoint_name: String,
    event_type: String,
    first_seen: String,
    last_seen: String,
    total_count: i64,
    pending_count: i64,
    severity: String,
    target_minutes: i64,
    acknowledged_at: Option<String>,
    overdue: bool,
}

async fn list_fingerprints(
    State(state): State<AppState>,
) -> Result<Json<Vec<FingerprintView>>, AppError> {
    let rows=sqlx::query("SELECT f.*,e.name endpoint_name FROM fingerprints f JOIN endpoints e ON e.id=f.endpoint_id ORDER BY CASE f.severity WHEN 'high' THEN 0 WHEN 'normal' THEN 1 ELSE 2 END,f.last_seen DESC LIMIT 200").fetch_all(&state.pool).await?;
    Ok(Json(rows.into_iter().map(fingerprint_from_row).collect()))
}

fn fingerprint_from_row(r: sqlx::sqlite::SqliteRow) -> FingerprintView {
    let last_seen: String = r.get("last_seen");
    let target: i64 = r.get("target_minutes");
    let ack: Option<String> = r.get("acknowledged_at");
    let overdue = r.get::<String, _>("severity") == "high"
        && ack.is_none()
        && DateTime::parse_from_rfc3339(&last_seen)
            .map(|d| d.with_timezone(&Utc) + ChronoDuration::minutes(target) < Utc::now())
            .unwrap_or(false);
    FingerprintView {
        fingerprint: r.get("fingerprint"),
        endpoint_id: r.get("endpoint_id"),
        endpoint_name: r.get("endpoint_name"),
        event_type: r.get("event_type"),
        first_seen: r.get("first_seen"),
        last_seen,
        total_count: r.get("total_count"),
        pending_count: r.get("pending_count"),
        severity: r.get("severity"),
        target_minutes: target,
        acknowledged_at: ack,
        overdue,
    }
}

#[derive(Serialize)]
struct FingerprintDetail {
    fingerprint: String,
    event_type: String,
    payload: Value,
    received_at: String,
    signature_valid: bool,
}
async fn fingerprint_detail(
    State(state): State<AppState>,
    Path(fp): Path<String>,
) -> Result<Json<FingerprintDetail>, AppError> {
    let row=sqlx::query("SELECT f.fingerprint,f.event_type,e.payload_encrypted,e.received_at,e.signature_valid FROM fingerprints f JOIN events e ON e.fingerprint=f.fingerprint WHERE f.fingerprint=? ORDER BY e.received_at DESC LIMIT 1").bind(&fp).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound)?;
    let encrypted: Vec<u8> = row.get("payload_encrypted");
    let plain = state.decrypt(&encrypted)?;
    let payload = serde_json::from_slice(&plain)
        .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&plain)}));
    Ok(Json(FingerprintDetail {
        fingerprint: row.get("fingerprint"),
        event_type: row.get("event_type"),
        payload,
        received_at: row.get("received_at"),
        signature_valid: row.get::<i64, _>("signature_valid") != 0,
    }))
}

#[derive(Deserialize)]
struct FingerprintUpdate {
    severity: String,
    target_minutes: i64,
}
async fn update_fingerprint(
    State(state): State<AppState>,
    Path(fp): Path<String>,
    Json(input): Json<FingerprintUpdate>,
) -> Result<StatusCode, AppError> {
    if !["normal", "high", "ignored"].contains(&input.severity.as_str()) {
        return Err(AppError::Invalid(
            "Severity must be normal, high, or ignored.".into(),
        ));
    }
    if !(1..=1440).contains(&input.target_minutes) {
        return Err(AppError::Invalid(
            "Target must be between 1 and 1,440 minutes.".into(),
        ));
    }
    let result=sqlx::query("UPDATE fingerprints SET severity=?,target_minutes=?,acknowledged_at=CASE WHEN ?='high' THEN NULL ELSE acknowledged_at END WHERE fingerprint=?").bind(&input.severity).bind(input.target_minutes).bind(&input.severity).bind(&fp).execute(&state.pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn ack_fingerprint(
    State(state): State<AppState>,
    Path(fp): Path<String>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query(
        "UPDATE fingerprints SET acknowledged_at=?,pending_count=0 WHERE fingerprint=?",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(fp)
    .execute(&state.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct SettingsView {
    quiet_start: String,
    quiet_end: String,
    utc_offset_minutes: i64,
    digest_minutes: i64,
    retention_days: i64,
    notification_configured: bool,
    notification_url: String,
    escalation_url: String,
    last_delivery_error: Option<String>,
}
async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsView>, AppError> {
    Ok(Json(load_settings(&state).await?))
}
async fn load_settings(state: &AppState) -> Result<SettingsView, AppError> {
    let r = sqlx::query("SELECT * FROM settings WHERE id=1")
        .fetch_one(&state.pool)
        .await?;
    let encrypted: Option<Vec<u8>> = r.get("notification_url_encrypted");
    let url = encrypted
        .as_ref()
        .map(|e| state.decrypt(e))
        .transpose()?
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();
    Ok(SettingsView {
        quiet_start: r.get("quiet_start"),
        quiet_end: r.get("quiet_end"),
        utc_offset_minutes: r.get("utc_offset_minutes"),
        digest_minutes: r.get("digest_minutes"),
        retention_days: r.get("retention_days"),
        notification_configured: !url.is_empty(),
        notification_url: url,
        escalation_url: r
            .get::<Option<String>, _>("escalation_url")
            .unwrap_or_default(),
        last_delivery_error: r.get("last_delivery_error"),
    })
}

#[derive(Deserialize)]
struct SettingsUpdate {
    quiet_start: String,
    quiet_end: String,
    utc_offset_minutes: i64,
    digest_minutes: i64,
    retention_days: i64,
    notification_url: String,
    escalation_url: String,
}
async fn update_settings(
    State(state): State<AppState>,
    Json(i): Json<SettingsUpdate>,
) -> Result<StatusCode, AppError> {
    validate_time(&i.quiet_start)?;
    validate_time(&i.quiet_end)?;
    if !(-720..=840).contains(&i.utc_offset_minutes) {
        return Err(AppError::Invalid("UTC offset is out of range.".into()));
    }
    if !(5..=1440).contains(&i.digest_minutes) {
        return Err(AppError::Invalid(
            "Digest interval must be 5–1,440 minutes.".into(),
        ));
    }
    if !(1..=365).contains(&i.retention_days) {
        return Err(AppError::Invalid("Retention must be 1–365 days.".into()));
    }
    for (name, url) in [
        ("Notification URL", i.notification_url.trim()),
        ("Escalation URL", i.escalation_url.trim()),
    ] {
        if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://localhost"))
        {
            return Err(AppError::Invalid(format!(
                "{name} must use HTTPS (localhost is allowed for development)."
            )));
        }
    }
    let encrypted = if i.notification_url.trim().is_empty() {
        None
    } else {
        Some(state.encrypt(i.notification_url.trim().as_bytes())?)
    };
    sqlx::query("UPDATE settings SET quiet_start=?,quiet_end=?,utc_offset_minutes=?,digest_minutes=?,retention_days=?,notification_url_encrypted=?,escalation_url=?,last_delivery_error=NULL,updated_at=? WHERE id=1")
        .bind(i.quiet_start).bind(i.quiet_end).bind(i.utc_offset_minutes).bind(i.digest_minutes).bind(i.retention_days).bind(encrypted).bind(if i.escalation_url.trim().is_empty(){None}else{Some(i.escalation_url.trim())}).bind(Utc::now().to_rfc3339()).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
fn validate_time(t: &str) -> Result<(), AppError> {
    let parts = t
        .split(':')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Invalid("Quiet hours must use HH:MM.".into()))?;
    if parts.len() != 2 || parts[0] > 23 || parts[1] > 59 {
        return Err(AppError::Invalid("Quiet hours must use HH:MM.".into()));
    }
    Ok(())
}

async fn notify_high(
    state: &AppState,
    fp: &str,
    event_type: &str,
    count: i64,
) -> Result<bool, AppError> {
    let settings = load_settings(state).await?;
    if settings.notification_url.is_empty() {
        return Ok(false);
    }
    let link = if settings.escalation_url.is_empty() {
        format!("{}/?fingerprint={}", state.public_url, fp)
    } else {
        settings.escalation_url
    };
    let text = format!(
        "🔴 High-severity webhook: {event_type} · {count} observation(s). Acknowledge: {link}"
    );
    deliver(state, &settings.notification_url, &text).await?;
    sqlx::query("UPDATE fingerprints SET pending_count=0,last_notified_at=? WHERE fingerprint=?")
        .bind(Utc::now().to_rfc3339())
        .bind(fp)
        .execute(&state.pool)
        .await?;
    Ok(true)
}

async fn deliver(state: &AppState, url: &str, text: &str) -> Result<(), AppError> {
    match state
        .http
        .post(url)
        .json(&json!({"text":text}))
        .send()
        .await
        .and_then(|r| r.error_for_status())
    {
        Ok(_) => {
            sqlx::query("UPDATE settings SET last_delivery_error=NULL WHERE id=1")
                .execute(&state.pool)
                .await?;
            Ok(())
        }
        Err(e) => {
            let msg = format!("Delivery failed: {}", e);
            sqlx::query("UPDATE settings SET last_delivery_error=? WHERE id=1")
                .bind(&msg)
                .execute(&state.pool)
                .await?;
            Err(AppError::Network(e))
        }
    }
}

pub async fn run_digest(state: &AppState, force: bool) -> Result<usize, AppError> {
    let settings = load_settings(state).await?;
    if settings.notification_url.is_empty() {
        return Ok(0);
    }
    if !force && is_quiet(&settings, Utc::now()) {
        return Ok(0);
    }
    let cutoff = (Utc::now() - ChronoDuration::minutes(settings.digest_minutes)).to_rfc3339();
    let rows=sqlx::query("SELECT fingerprint,event_type,pending_count FROM fingerprints WHERE severity='normal' AND pending_count>0 AND (? OR last_notified_at IS NULL OR last_notified_at<?) ORDER BY pending_count DESC LIMIT 12").bind(force).bind(cutoff).fetch_all(&state.pool).await?;
    if rows.is_empty() {
        return Ok(0);
    }
    let count = rows.len();
    let observations: i64 = rows.iter().map(|r| r.get::<i64, _>("pending_count")).sum();
    let mut lines = rows
        .iter()
        .take(5)
        .map(|r| {
            format!(
                "• {} × {}",
                r.get::<String, _>("event_type"),
                r.get::<i64, _>("pending_count")
            )
        })
        .collect::<Vec<_>>();
    if count > 5 {
        lines.push(format!("• and {} more fingerprints", count - 5));
    }
    let link = if settings.escalation_url.is_empty() {
        state.public_url.clone()
    } else {
        settings.escalation_url
    };
    let text=format!("🌿 Webhook digest · {observations} observations in {count} fingerprints\n{}\nReview once: {link}",lines.join("\n"));
    deliver(state, &settings.notification_url, &text).await?;
    let now = Utc::now().to_rfc3339();
    for r in rows {
        sqlx::query(
            "UPDATE fingerprints SET pending_count=0,last_notified_at=? WHERE fingerprint=?",
        )
        .bind(&now)
        .bind(r.get::<String, _>("fingerprint"))
        .execute(&state.pool)
        .await?;
    }
    Ok(count)
}
fn is_quiet(s: &SettingsView, now: DateTime<Utc>) -> bool {
    let local = now + ChronoDuration::minutes(s.utc_offset_minutes);
    let mins = local.hour() * 60 + local.minute();
    let parse = |v: &str| {
        let mut p = v.split(':').filter_map(|x| x.parse::<u32>().ok());
        p.next().unwrap_or(0) * 60 + p.next().unwrap_or(0)
    };
    let start = parse(&s.quiet_start);
    let end = parse(&s.quiet_end);
    if start == end {
        false
    } else if start < end {
        mins >= start && mins < end
    } else {
        mins >= start || mins < end
    }
}

async fn send_digest_now(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let count = run_digest(&state, true).await?;
    Ok(Json(json!({"sent":count})))
}

async fn export_csv(State(state): State<AppState>) -> Result<Response, AppError> {
    let rows=sqlx::query("SELECT e.name,f.fingerprint,f.event_type,f.severity,f.total_count,f.pending_count,f.first_seen,f.last_seen,f.acknowledged_at FROM fingerprints f JOIN endpoints e ON e.id=f.endpoint_id ORDER BY f.last_seen DESC").fetch_all(&state.pool).await?;
    let mut csv="alias,fingerprint,event_type,severity,total_count,pending_count,first_seen,last_seen,acknowledged_at\n".to_string();
    for r in rows {
        let cells = [
            r.get::<String, _>("name"),
            r.get("fingerprint"),
            r.get("event_type"),
            r.get("severity"),
            r.get::<i64, _>("total_count").to_string(),
            r.get::<i64, _>("pending_count").to_string(),
            r.get("first_seen"),
            r.get("last_seen"),
            r.get::<Option<String>, _>("acknowledged_at")
                .unwrap_or_default(),
        ];
        csv.push_str(
            &cells
                .iter()
                .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=webhook-fingerprints.csv",
            ),
        ],
        csv,
    )
        .into_response())
}

pub fn spawn_maintenance(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = run_digest(&state, false).await {
                warn!(error=%e,"digest cycle failed")
            }
            if let Err(e) = cleanup(&state).await {
                warn!(error=%e,"retention cleanup failed")
            }
        }
    });
}
async fn cleanup(state: &AppState) -> Result<(), AppError> {
    let days: i64 = sqlx::query_scalar("SELECT retention_days FROM settings WHERE id=1")
        .fetch_one(&state.pool)
        .await?;
    let cutoff = (Utc::now() - ChronoDuration::days(days)).to_rfc3339();
    sqlx::query("DELETE FROM events WHERE received_at<?")
        .bind(cutoff)
        .execute(&state.pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use tower::ServiceExt;
    async fn state() -> AppState {
        let dir = tempfile::tempdir().unwrap().keep();
        let cfg = AppConfig {
            database_url: format!("sqlite://{}?mode=rwc", dir.join("test.db").display()),
            admin_token: "test-token".into(),
            encryption_key: [7; 32],
            public_url: "http://localhost".into(),
            build_sha: "test".into(),
        };
        AppState::connect(&cfg).await.unwrap()
    }
    #[tokio::test]
    async fn encrypts_and_decrypts_payloads() {
        let s = state().await;
        let a = s.encrypt(b"secret").unwrap();
        let b = s.encrypt(b"secret").unwrap();
        assert_ne!(a, b);
        assert_eq!(s.decrypt(&a).unwrap(), b"secret");
    }
    #[test]
    fn fingerprint_is_stable_across_json_order() {
        let a = json!({"type":"failed","status":500,"id":1});
        let b = json!({"id":2,"status":500,"type":"failed"});
        assert_eq!(
            make_fingerprint(1, "failed", &a),
            make_fingerprint(1, "failed", &b)
        );
    }
    #[test]
    fn overnight_quiet_window_works() {
        let mut s = SettingsView {
            quiet_start: "22:00".into(),
            quiet_end: "07:00".into(),
            utc_offset_minutes: 0,
            digest_minutes: 60,
            retention_days: 7,
            notification_configured: false,
            notification_url: "".into(),
            escalation_url: "".into(),
            last_delivery_error: None,
        };
        let at = DateTime::parse_from_rfc3339("2026-08-27T23:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(is_quiet(&s, at));
        s.quiet_start = "09:00".into();
        s.quiet_end = "17:00".into();
        assert!(!is_quiet(&s, at));
    }
    #[tokio::test]
    async fn migrations_create_required_settings() {
        let s = state().await;
        let days: i64 = sqlx::query_scalar("SELECT retention_days FROM settings")
            .fetch_one(&s.pool)
            .await
            .unwrap();
        assert_eq!(days, 7);
    }

    #[tokio::test]
    async fn signed_webhook_is_received_and_grouped() {
        let state = state().await;
        let dist = tempfile::tempdir().unwrap();
        let app = build_app(state, dist.path().to_path_buf());
        let create = Request::builder()
            .method("POST")
            .uri("/api/endpoints")
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"name":"Billing","require_signature":true}"#))
            .unwrap();
        let created = app.clone().oneshot(create).await.unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let json: Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        let secret = json["signing_secret"].as_str().unwrap();
        let body = br#"{"type":"invoice.failed","status":500,"id":"evt_1"}"#;
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        let hook_path = json["hook_url"]
            .as_str()
            .unwrap()
            .strip_prefix("http://localhost")
            .unwrap();
        let receive = Request::builder()
            .method("POST")
            .uri(hook_path)
            .header("x-webhook-signature", signature)
            .body(Body::from(body.as_slice()))
            .unwrap();
        let accepted = app.clone().oneshot(receive).await.unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        let list = Request::builder()
            .uri("/api/fingerprints")
            .header(header::AUTHORIZATION, "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(list).await.unwrap();
        let json: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(json[0]["event_type"], "invoice.failed");
        assert_eq!(json[0]["total_count"], 1);
    }
}

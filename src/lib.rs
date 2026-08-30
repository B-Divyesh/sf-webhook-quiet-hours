use std::{
    collections::{BTreeSet, HashMap},
    env, fs,
    io::Write,
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::Duration,
};

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
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorError,
    GovernorLayer,
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
    secret_sources: SecretSources,
    secret_directory: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SecretSource {
    Supplied,
    Persisted,
    Generated,
}

impl SecretSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Supplied => "supplied",
            Self::Persisted => "persisted",
            Self::Generated => "generated",
        }
    }
}

#[derive(Clone, Copy)]
struct SecretSources {
    admin_token: SecretSource,
    encryption_key: SecretSource,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://data/quiet-hours.db?mode=rwc".into());
        let secret_directory = secret_directory_for_database(&database_url);
        let (admin_token, admin_source) = resolve_admin_token(&secret_directory)?;
        let (encryption_key, encryption_source) = resolve_encryption_key(&secret_directory)?;
        Ok(Self {
            database_url,
            admin_token,
            encryption_key,
            public_url: env::var("PUBLIC_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into())
                .trim_end_matches('/')
                .into(),
            build_sha: env::var("BUILD_SHA").unwrap_or_else(|_| "development".into()),
            secret_sources: SecretSources {
                admin_token: admin_source,
                encryption_key: encryption_source,
            },
            secret_directory,
        })
    }

    pub fn admin_token_source(&self) -> &'static str {
        self.secret_sources.admin_token.as_str()
    }

    pub fn encryption_key_source(&self) -> &'static str {
        self.secret_sources.encryption_key.as_str()
    }

    pub fn secret_directory(&self) -> &FsPath {
        &self.secret_directory
    }
}

fn secret_directory_for_database(database_url: &str) -> PathBuf {
    database_url
        .strip_prefix("sqlite://")
        .and_then(|value| value.split('?').next())
        .and_then(|value| FsPath::new(value).parent())
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(FsPath::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("data"))
}

fn resolve_admin_token(directory: &FsPath) -> Result<(String, SecretSource), AppError> {
    match env::var("ADMIN_TOKEN") {
        Ok(value) if value.trim().is_empty() => Err(AppError::Config(
            "ADMIN_TOKEN must not be empty when supplied".into(),
        )),
        Ok(value) => Ok((value, SecretSource::Supplied)),
        Err(env::VarError::NotUnicode(_)) => Err(AppError::Config(
            "ADMIN_TOKEN must contain valid UTF-8".into(),
        )),
        Err(env::VarError::NotPresent) => {
            let path = directory.join("admin-token");
            if path.exists() {
                let value = fs::read_to_string(&path)?.trim().to_owned();
                if value.is_empty() {
                    return Err(AppError::Config(format!(
                        "persisted admin token is empty: {}",
                        path.display()
                    )));
                }
                return Ok((value, SecretSource::Persisted));
            }
            let mut random = [0_u8; 32];
            OsRng.fill_bytes(&mut random);
            let value = hex::encode(random);
            persist_secret(&path, &value)?;
            Ok((value, SecretSource::Generated))
        }
    }
}

fn resolve_encryption_key(directory: &FsPath) -> Result<([u8; 32], SecretSource), AppError> {
    match env::var("DATA_ENCRYPTION_KEY") {
        Ok(value) => Ok((decode_encryption_key(&value)?, SecretSource::Supplied)),
        Err(env::VarError::NotUnicode(_)) => Err(AppError::Config(
            "DATA_ENCRYPTION_KEY must contain valid UTF-8".into(),
        )),
        Err(env::VarError::NotPresent) => {
            let path = directory.join("encryption-key");
            if path.exists() {
                let value = fs::read_to_string(&path)?;
                return Ok((
                    decode_encryption_key(value.trim()).map_err(|_| {
                        AppError::Config(format!(
                            "persisted encryption key is invalid: {}",
                            path.display()
                        ))
                    })?,
                    SecretSource::Persisted,
                ));
            }
            let mut key = [0_u8; 32];
            OsRng.fill_bytes(&mut key);
            persist_secret(&path, &BASE64.encode(key))?;
            Ok((key, SecretSource::Generated))
        }
    }
}

fn decode_encryption_key(value: &str) -> Result<[u8; 32], AppError> {
    let decoded = BASE64
        .decode(value)
        .map_err(|_| AppError::Config("DATA_ENCRYPTION_KEY must be base64".into()))?;
    decoded
        .try_into()
        .map_err(|_| AppError::Config("DATA_ENCRYPTION_KEY must decode to exactly 32 bytes".into()))
}

fn persist_secret(path: &FsPath, value: &str) -> Result<(), AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Config("secret path has no parent directory".into()))?;
    fs::create_dir_all(parent)?;
    let mut suffix = [0_u8; 8];
    OsRng.fill_bytes(&mut suffix);
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("secret"),
        hex::encode(suffix)
    ));

    let write_result = (|| -> Result<(), std::io::Error> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(value.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result.map_err(AppError::Io)
}

#[derive(Clone)]
pub struct AppState {
    pool: SqlitePool,
    cipher: Arc<Aes256Gcm>,
    admin_hash: [u8; 32],
    public_url: String,
    build_sha: String,
    http: reqwest::Client,
    demos: Arc<RwLock<HashMap<String, DemoWorkspace>>>,
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
            demos: Arc::new(RwLock::new(HashMap::new())),
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
    // Bound accidental/hostile floods before parsing or encrypting request bodies. SmartIp uses
    // the first X-Forwarded-For hop supplied by the factory ingress, then falls back to the
    // direct peer for local deployments.
    let hook_limit = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_millisecond(10)
            .burst_size(200)
            .error_handler(rate_limit_response)
            .finish()
            .expect("valid webhook rate limit"),
    );
    let api_limit = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(20)
            .burst_size(40)
            .error_handler(rate_limit_response)
            .finish()
            .expect("valid API rate limit"),
    );
    let demo_limit = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_millisecond(10)
            .burst_size(200)
            .error_handler(rate_limit_response)
            .finish()
            .expect("valid demo rate limit"),
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
        .fallback(api_not_found)
        .layer(middleware::from_fn_with_state(state.clone(), admin_auth))
        // The governor must wrap authentication so invalid credentials consume
        // the same per-client allowance as authenticated API traffic.
        .layer(GovernorLayer {
            config: api_limit.clone(),
        });
    let demo_api = Router::new()
        .route("/demo/session", post(create_demo_session))
        .route("/demo/:workspace/session", delete(discard_demo_session))
        .route("/demo/:workspace/reset", post(reset_demo_session))
        .route("/demo/:workspace/summary", get(demo_summary))
        .route("/demo/:workspace/endpoints", get(demo_endpoints))
        .route(
            "/demo/:workspace/endpoints/:id",
            delete(demo_remove_endpoint),
        )
        .route("/demo/:workspace/fingerprints", get(demo_fingerprints))
        .route(
            "/demo/:workspace/fingerprints/:fingerprint",
            get(demo_fingerprint_detail).patch(demo_update_fingerprint),
        )
        .route(
            "/demo/:workspace/fingerprints/:fingerprint/ack",
            post(demo_ack_fingerprint),
        )
        .route(
            "/demo/:workspace/settings",
            get(demo_get_settings).put(demo_update_settings),
        )
        .route("/demo/:workspace/digest/send", post(demo_send_digest))
        .route("/demo/:workspace/export.csv", get(demo_export_csv))
        .layer(GovernorLayer { config: demo_limit });
    // Vite fingerprints compiled JS/CSS. Keep those bytes for a year while
    // ensuring the HTML shell and service worker always revalidate, so a
    // deploy can publish an updated manifest without clients being stranded
    // on an old shell.
    let static_service = ServiceBuilder::new()
        .layer(middleware::from_fn(static_cache_control))
        .service(ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html"))));
    Router::new()
        .route("/health", get(health))
        .merge(hook)
        .nest("/api", api.merge(demo_api))
        .fallback_service(static_service)
        .layer(SetResponseHeaderLayer::if_not_present(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")))
        .layer(SetResponseHeaderLayer::if_not_present(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
        .layer(SetResponseHeaderLayer::if_not_present(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer")))
        .layer(SetResponseHeaderLayer::if_not_present(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; connect-src 'self' https://api.sociobot.in https://pilot-api.sociobot.in; frame-ancestors 'none'; base-uri 'none'; form-action 'self' https://api.sociobot.in https://pilot-api.sociobot.in")))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn rate_limit_response(error: GovernorError) -> Response {
    let retry_after = match error {
        GovernorError::TooManyRequests { wait_time, .. } => wait_time.max(1),
        _ => 1,
    };
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({"error":"Too many requests. Try again shortly."})),
    )
        .into_response();
    response.headers_mut().insert(
        header::RETRY_AFTER,
        HeaderValue::from_str(&retry_after.to_string()).expect("valid Retry-After value"),
    );
    response
}

async fn static_cache_control(req: Request<Body>, next: Next) -> Response {
    let path = req.uri().path().to_owned();
    let mut response = next.run(req).await;
    if response.status().is_success() {
        // ServeDir's SPA fallback returns index.html for an absent asset too.
        // Never give that HTML an asset lifetime merely because the requested
        // pathname looked fingerprinted.
        let cache_control = if response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|content_type| content_type.starts_with("text/html"))
        {
            "no-cache"
        } else {
            static_cache_control_value(&path)
        };
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static(cache_control),
        );
        if response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|content_type| content_type.starts_with("text/html"))
            && !matches!(path.as_str(), "/" | "/demo" | "/privacy" | "/terms")
        {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }
    response
}

fn static_cache_control_value(path: &str) -> &'static str {
    if is_fingerprinted_asset(path) {
        "public, max-age=31536000, immutable"
    } else if path.starts_with("/assets/") {
        // Public images are not Vite-fingerprinted, but are small and may be
        // safely refreshed daily when an operator replaces one.
        "public, max-age=86400"
    } else {
        // This covers the SPA shell (including deep links) and /sw.js.
        "no-cache"
    }
}

fn is_fingerprinted_asset(path: &str) -> bool {
    if !path.starts_with("/assets/") {
        return false;
    }
    let Some(file_name) = path.rsplit('/').next() else {
        return false;
    };
    let Some((stem, _extension)) = file_name.rsplit_once('.') else {
        return false;
    };
    let bytes = stem.as_bytes();
    bytes.len() >= 9
        && bytes[bytes.len() - 9] == b'-'
        && bytes[bytes.len() - 8..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
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

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({"error":"That API route does not exist."})),
    )
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

#[derive(Clone, Serialize)]
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

#[derive(Clone, Serialize)]
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

#[derive(Clone, Serialize)]
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
    if input.severity == "high" {
        let row =
            sqlx::query("SELECT event_type,total_count FROM fingerprints WHERE fingerprint=?")
                .bind(&fp)
                .fetch_one(&state.pool)
                .await?;
        notify_high(
            &state,
            &fp,
            &row.get::<String, _>("event_type"),
            row.get("total_count"),
        )
        .await?;
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

#[derive(Clone, Serialize)]
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

#[derive(Clone)]
struct DemoWorkspace {
    created_at: DateTime<Utc>,
    endpoints: Vec<EndpointView>,
    fingerprints: Vec<FingerprintView>,
    details: HashMap<String, FingerprintDetail>,
    settings: SettingsView,
}

#[derive(Serialize)]
struct DemoSession {
    workspace_id: String,
    expires_at: String,
    summary: Summary,
    endpoints: Vec<EndpointView>,
    fingerprints: Vec<FingerprintView>,
    details: HashMap<String, FingerprintDetail>,
    settings: SettingsView,
}

fn seeded_demo_workspace() -> DemoWorkspace {
    let now = Utc::now();
    let endpoint = EndpointView {
        id: 1,
        slug: "demo-deploy-monitor".into(),
        name: "Deploy monitor".into(),
        signature_required: true,
        created_at: (now - ChronoDuration::days(12)).to_rfc3339(),
    };
    let samples = [
        (
            "demo-deploy-failed",
            "deployment.failed",
            6,
            2,
            "high",
            json!({"type":"deployment.failed","status":500,"service":"checkout-api","region":"eu-west"}),
        ),
        (
            "demo-invoice-sync",
            "invoice.sync.failed",
            9,
            9,
            "normal",
            json!({"type":"invoice.sync.failed","status":503,"service":"billing-worker","retry":3}),
        ),
        (
            "demo-backup-complete",
            "backup.completed",
            3,
            0,
            "ignored",
            json!({"type":"backup.completed","status":200,"service":"nightly-backup"}),
        ),
    ];
    let mut fingerprints = Vec::new();
    let mut details = HashMap::new();
    for (index, (fingerprint, event_type, total_count, pending_count, severity, payload)) in
        samples.into_iter().enumerate()
    {
        let received_at = now - ChronoDuration::minutes((index as i64 + 1) * 7);
        fingerprints.push(FingerprintView {
            fingerprint: fingerprint.into(),
            endpoint_id: endpoint.id,
            endpoint_name: endpoint.name.clone(),
            event_type: event_type.into(),
            first_seen: (received_at - ChronoDuration::hours(4)).to_rfc3339(),
            last_seen: received_at.to_rfc3339(),
            total_count,
            pending_count,
            severity: severity.into(),
            target_minutes: 30,
            acknowledged_at: None,
            overdue: false,
        });
        details.insert(
            fingerprint.into(),
            FingerprintDetail {
                fingerprint: fingerprint.into(),
                event_type: event_type.into(),
                payload,
                received_at: received_at.to_rfc3339(),
                signature_valid: true,
            },
        );
    }
    DemoWorkspace {
        created_at: now,
        endpoints: vec![endpoint],
        fingerprints,
        details,
        settings: SettingsView {
            quiet_start: "22:00".into(),
            quiet_end: "08:00".into(),
            utc_offset_minutes: 0,
            digest_minutes: 60,
            retention_days: 7,
            notification_configured: false,
            notification_url: String::new(),
            escalation_url: "https://status.example.test/incidents".into(),
            last_delivery_error: None,
        },
    }
}

async fn demo_snapshot(state: &AppState, workspace: &str) -> Result<DemoWorkspace, AppError> {
    let mut demos = state.demos.write().await;
    let cutoff = Utc::now() - ChronoDuration::hours(24);
    demos.retain(|_, demo| demo.created_at > cutoff);
    demos.get(workspace).cloned().ok_or(AppError::NotFound)
}

async fn create_demo_session(State(state): State<AppState>) -> Json<DemoSession> {
    let workspace_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let workspace = seeded_demo_workspace();
    let expires_at = (workspace.created_at + ChronoDuration::hours(24)).to_rfc3339();
    let session = DemoSession {
        workspace_id: workspace_id.clone(),
        expires_at,
        summary: demo_summary_value(&workspace),
        endpoints: workspace.endpoints.clone(),
        fingerprints: workspace.fingerprints.clone(),
        details: workspace.details.clone(),
        settings: workspace.settings.clone(),
    };
    state
        .demos
        .write()
        .await
        .insert(workspace_id.clone(), workspace);
    Json(session)
}

async fn discard_demo_session(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<StatusCode, AppError> {
    state
        .demos
        .write()
        .await
        .remove(&workspace)
        .ok_or(AppError::NotFound)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_demo_session(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<DemoSession>, AppError> {
    let mut demos = state.demos.write().await;
    if !demos.contains_key(&workspace) {
        return Err(AppError::NotFound);
    }
    let seeded = seeded_demo_workspace();
    let expires_at = (seeded.created_at + ChronoDuration::hours(24)).to_rfc3339();
    let session = DemoSession {
        workspace_id: workspace.clone(),
        expires_at,
        summary: demo_summary_value(&seeded),
        endpoints: seeded.endpoints.clone(),
        fingerprints: seeded.fingerprints.clone(),
        details: seeded.details.clone(),
        settings: seeded.settings.clone(),
    };
    demos.insert(workspace.clone(), seeded);
    Ok(Json(session))
}

fn demo_summary_value(workspace: &DemoWorkspace) -> Summary {
    let total: i64 = workspace
        .fingerprints
        .iter()
        .map(|fingerprint| fingerprint.total_count)
        .sum();
    Summary {
        endpoints: workspace.endpoints.len() as i64,
        fingerprints: workspace.fingerprints.len() as i64,
        events_today: total,
        pending: workspace
            .fingerprints
            .iter()
            .filter(|fingerprint| fingerprint.severity != "ignored")
            .map(|fingerprint| fingerprint.pending_count)
            .sum(),
        compressed: (total - workspace.fingerprints.len() as i64).max(0),
        high_unacknowledged: workspace
            .fingerprints
            .iter()
            .filter(|fingerprint| {
                fingerprint.severity == "high" && fingerprint.acknowledged_at.is_none()
            })
            .count() as i64,
    }
}

async fn demo_summary(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<Summary>, AppError> {
    Ok(Json(demo_summary_value(
        &demo_snapshot(&state, &workspace).await?,
    )))
}

async fn demo_endpoints(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<Vec<EndpointView>>, AppError> {
    Ok(Json(demo_snapshot(&state, &workspace).await?.endpoints))
}

async fn demo_fingerprints(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<Vec<FingerprintView>>, AppError> {
    Ok(Json(demo_snapshot(&state, &workspace).await?.fingerprints))
}

async fn demo_fingerprint_detail(
    State(state): State<AppState>,
    Path((workspace, fingerprint)): Path<(String, String)>,
) -> Result<Json<FingerprintDetail>, AppError> {
    demo_snapshot(&state, &workspace)
        .await?
        .details
        .get(&fingerprint)
        .cloned()
        .map(Json)
        .ok_or(AppError::NotFound)
}

async fn demo_ack_fingerprint(
    State(state): State<AppState>,
    Path((workspace, fingerprint)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let mut demos = state.demos.write().await;
    let demo = demos.get_mut(&workspace).ok_or(AppError::NotFound)?;
    let item = demo
        .fingerprints
        .iter_mut()
        .find(|item| item.fingerprint == fingerprint)
        .ok_or(AppError::NotFound)?;
    item.acknowledged_at = Some(Utc::now().to_rfc3339());
    item.pending_count = 0;
    Ok(StatusCode::NO_CONTENT)
}

async fn demo_update_fingerprint(
    State(state): State<AppState>,
    Path((workspace, fingerprint)): Path<(String, String)>,
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
    let mut demos = state.demos.write().await;
    let demo = demos.get_mut(&workspace).ok_or(AppError::NotFound)?;
    let item = demo
        .fingerprints
        .iter_mut()
        .find(|item| item.fingerprint == fingerprint)
        .ok_or(AppError::NotFound)?;
    item.severity = input.severity;
    item.target_minutes = input.target_minutes;
    item.acknowledged_at = None;
    Ok(StatusCode::NO_CONTENT)
}

async fn demo_remove_endpoint(
    State(state): State<AppState>,
    Path((workspace, id)): Path<(String, i64)>,
) -> Result<StatusCode, AppError> {
    let mut demos = state.demos.write().await;
    let demo = demos.get_mut(&workspace).ok_or(AppError::NotFound)?;
    let before = demo.endpoints.len();
    demo.endpoints.retain(|endpoint| endpoint.id != id);
    if demo.endpoints.len() == before {
        return Err(AppError::NotFound);
    }
    demo.fingerprints.retain(|item| item.endpoint_id != id);
    let remaining = demo
        .fingerprints
        .iter()
        .map(|item| item.fingerprint.clone())
        .collect::<BTreeSet<_>>();
    demo.details
        .retain(|fingerprint, _| remaining.contains(fingerprint));
    Ok(StatusCode::NO_CONTENT)
}

async fn demo_get_settings(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<SettingsView>, AppError> {
    Ok(Json(demo_snapshot(&state, &workspace).await?.settings))
}

async fn demo_update_settings(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
    Json(input): Json<SettingsUpdate>,
) -> Result<StatusCode, AppError> {
    validate_time(&input.quiet_start)?;
    validate_time(&input.quiet_end)?;
    if !(-720..=840).contains(&input.utc_offset_minutes)
        || !(5..=1440).contains(&input.digest_minutes)
        || !(1..=90).contains(&input.retention_days)
    {
        return Err(AppError::Invalid("A demo setting is out of range.".into()));
    }
    let mut demos = state.demos.write().await;
    let demo = demos.get_mut(&workspace).ok_or(AppError::NotFound)?;
    demo.settings = SettingsView {
        quiet_start: input.quiet_start,
        quiet_end: input.quiet_end,
        utc_offset_minutes: input.utc_offset_minutes,
        digest_minutes: input.digest_minutes,
        retention_days: input.retention_days,
        notification_configured: false,
        // Demo notification destinations are deliberately never retained or called.
        notification_url: String::new(),
        escalation_url: input.escalation_url,
        last_delivery_error: None,
    };
    Ok(StatusCode::NO_CONTENT)
}

async fn demo_send_digest(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Json<Value>, AppError> {
    let mut demos = state.demos.write().await;
    let demo = demos.get_mut(&workspace).ok_or(AppError::NotFound)?;
    let mut sent = 0;
    for item in &mut demo.fingerprints {
        if item.severity == "normal" && item.pending_count > 0 {
            sent += 1;
            item.pending_count = 0;
        }
    }
    Ok(Json(json!({"sent":sent})))
}

async fn demo_export_csv(
    State(state): State<AppState>,
    Path(workspace): Path<String>,
) -> Result<Response, AppError> {
    let demo = demo_snapshot(&state, &workspace).await?;
    let mut csv = "alias,fingerprint,event_type,severity,total_count,pending_count,first_seen,last_seen,acknowledged_at\n".to_string();
    for item in demo.fingerprints {
        let cells = [
            item.endpoint_name,
            item.fingerprint,
            item.event_type,
            item.severity,
            item.total_count.to_string(),
            item.pending_count.to_string(),
            item.first_seen,
            item.last_seen,
            item.acknowledged_at.unwrap_or_default(),
        ];
        csv.push_str(
            &cells
                .iter()
                .map(|cell| format!("\"{}\"", cell.replace('"', "\"\"")))
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
                "attachment; filename=webhook-fingerprints-demo.csv",
            ),
        ],
        csv,
    )
        .into_response())
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
    let rows=sqlx::query("SELECT fingerprint,event_type,pending_count FROM fingerprints WHERE severity='normal' AND pending_count>0 AND (? OR (last_notified_at IS NULL AND first_seen<?) OR last_notified_at<?) ORDER BY pending_count DESC LIMIT 12").bind(force).bind(&cutoff).bind(&cutoff).fetch_all(&state.pool).await?;
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
            secret_sources: SecretSources {
                admin_token: SecretSource::Supplied,
                encryption_key: SecretSource::Supplied,
            },
            secret_directory: dir.clone(),
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
    // @claim:quiet-window
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

    // @claim:signed-ingress
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
            .header("x-forwarded-for", "203.0.113.10")
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
            .header("x-forwarded-for", "203.0.113.10")
            .body(Body::from(body.as_slice()))
            .unwrap();
        let accepted = app.clone().oneshot(receive).await.unwrap();
        assert_eq!(accepted.status(), StatusCode::ACCEPTED);
        let list = Request::builder()
            .uri("/api/fingerprints")
            .header(header::AUTHORIZATION, "Bearer test-token")
            .header("x-forwarded-for", "203.0.113.10")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(list).await.unwrap();
        let json: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(json[0]["event_type"], "invoice.failed");
        assert_eq!(json[0]["total_count"], 1);
    }

    // @claim:encrypted-payloads
    #[tokio::test]
    async fn claim_payloads_are_encrypted_at_rest() {
        let state = state().await;
        let plaintext = br#"{"type":"private.failed","secret_marker":"never-store-plain"}"#;
        let encrypted = state.encrypt(plaintext).unwrap();
        sqlx::query("INSERT INTO endpoints(slug,name,token_hash,created_at) VALUES(?,?,?,?)")
            .bind("claim-endpoint")
            .bind("Claim endpoint")
            .bind("hash")
            .bind(Utc::now().to_rfc3339())
            .execute(&state.pool)
            .await
            .unwrap();
        let endpoint_id: i64 = sqlx::query_scalar("SELECT id FROM endpoints LIMIT 1")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO fingerprints(fingerprint,endpoint_id,event_type,first_seen,last_seen,total_count,pending_count) VALUES(?,?,?,?,?,1,1)")
            .bind("claim-fingerprint")
            .bind(endpoint_id)
            .bind("private.failed")
            .bind(Utc::now().to_rfc3339())
            .bind(Utc::now().to_rfc3339())
            .execute(&state.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO events(endpoint_id,fingerprint,event_type,received_at,payload_encrypted,signature_valid) VALUES(?,?,?,?,?,1)")
            .bind(endpoint_id)
            .bind("claim-fingerprint")
            .bind("private.failed")
            .bind(Utc::now().to_rfc3339())
            .bind(&encrypted)
            .execute(&state.pool)
            .await
            .unwrap();

        let stored: Vec<u8> = sqlx::query_scalar("SELECT payload_encrypted FROM events LIMIT 1")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_ne!(stored, plaintext);
        assert!(!String::from_utf8_lossy(&stored).contains("never-store-plain"));
        assert_eq!(state.decrypt(&stored).unwrap(), plaintext);
    }

    #[tokio::test]
    async fn static_files_have_update_safe_cache_headers() {
        let state = state().await;
        let dist = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dist.path().join("assets")).unwrap();
        std::fs::write(dist.path().join("index.html"), "<main>shell</main>").unwrap();
        std::fs::write(
            dist.path().join("sw.js"),
            "self.addEventListener('fetch', () => {})",
        )
        .unwrap();
        std::fs::write(dist.path().join("assets/index-Ab12_cd3.js"), "export {};").unwrap();
        std::fs::write(dist.path().join("assets/moon-bloom-480.webp"), "image").unwrap();
        let app = build_app(state, dist.path().to_path_buf());

        for (uri, expected_status, expected_cache) in [
            (
                "/assets/index-Ab12_cd3.js",
                StatusCode::OK,
                "public, max-age=31536000, immutable",
            ),
            (
                "/assets/moon-bloom-480.webp",
                StatusCode::OK,
                "public, max-age=86400",
            ),
            (
                "/assets/missing-Ab12_cd3.js",
                StatusCode::NOT_FOUND,
                "no-cache",
            ),
            ("/sw.js", StatusCode::OK, "no-cache"),
            ("/privacy", StatusCode::OK, "no-cache"),
            ("/missing-page", StatusCode::NOT_FOUND, "no-cache"),
        ] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected_status, "{uri}");
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                expected_cache,
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn api_rate_limit_uses_first_forwarded_hop_and_sets_retry_after() {
        let state = state().await;
        let dist = tempfile::tempdir().unwrap();
        let app = build_app(state, dist.path().to_path_buf());
        let request_for = |ip: &str| {
            Request::builder()
                .uri("/api/summary")
                .header(header::AUTHORIZATION, "Bearer test-token")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap()
        };

        let mut limited = None;
        for _ in 0..100 {
            let response = app
                .clone()
                .oneshot(request_for("203.0.113.77"))
                .await
                .unwrap();
            if response.status() == StatusCode::TOO_MANY_REQUESTS {
                limited = Some(response);
                break;
            }
            assert_eq!(response.status(), StatusCode::OK);
        }
        let limited = limited.expect("one client should be rate limited after its burst");
        assert!(limited.headers().contains_key(header::RETRY_AFTER));
        assert_ne!(
            limited.headers().get(header::RETRY_AFTER).unwrap(),
            "0",
            "Retry-After must ask clients to wait at least one second"
        );

        let other_client = app
            .oneshot(request_for("203.0.113.78, 10.0.0.1"))
            .await
            .unwrap();
        assert_eq!(other_client.status(), StatusCode::OK);
    }

    // @claim:api-rate-limit
    #[tokio::test]
    async fn unauthenticated_api_requests_are_rate_limited_before_authentication() {
        let state = state().await;
        let dist = tempfile::tempdir().unwrap();
        let app = build_app(state, dist.path().to_path_buf());
        let request_for = |ip: &str| {
            Request::builder()
                .uri("/api/summary")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap()
        };

        let mut unauthorized = 0;
        let mut limited = None;
        for _ in 0..100 {
            let response = app
                .clone()
                .oneshot(request_for("203.0.113.91"))
                .await
                .unwrap();
            match response.status() {
                StatusCode::UNAUTHORIZED => unauthorized += 1,
                StatusCode::TOO_MANY_REQUESTS => {
                    limited = Some(response);
                    break;
                }
                status => panic!("unexpected pre-auth response: {status}"),
            }
        }
        assert!(unauthorized > 0);
        let limited = limited.expect("invalid credentials must not bypass the API governor");
        assert!(limited.headers().contains_key(header::RETRY_AFTER));

        let other_client = app.oneshot(request_for("203.0.113.92")).await.unwrap();
        assert_eq!(other_client.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn demo_workspaces_are_seeded_isolated_resettable_and_never_touch_real_data() {
        let state = state().await;
        let dist = tempfile::tempdir().unwrap();
        let app = build_app(state, dist.path().to_path_buf());
        let request = |method: &str, uri: &str| {
            Request::builder()
                .method(method)
                .uri(uri)
                .header("x-forwarded-for", "203.0.113.111")
                .body(Body::empty())
                .unwrap()
        };

        let created = app
            .clone()
            .oneshot(request("POST", "/api/demo/session"))
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);
        let created: Value =
            serde_json::from_slice(&to_bytes(created.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        let workspace = created["workspace_id"].as_str().unwrap();

        let demo_summary = app
            .clone()
            .oneshot(request("GET", &format!("/api/demo/{workspace}/summary")))
            .await
            .unwrap();
        assert_eq!(demo_summary.status(), StatusCode::OK);
        let demo_summary: Value =
            serde_json::from_slice(&to_bytes(demo_summary.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(demo_summary["events_today"], 18);
        assert_eq!(demo_summary["fingerprints"], 3);
        assert_eq!(demo_summary["compressed"], 15);

        let ack = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/api/demo/{workspace}/fingerprints/demo-deploy-failed/ack"),
            ))
            .await
            .unwrap();
        assert_eq!(ack.status(), StatusCode::NO_CONTENT);

        let real_summary = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/summary")
                    .header(header::AUTHORIZATION, "Bearer test-token")
                    .header("x-forwarded-for", "203.0.113.112")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let real_summary: Value =
            serde_json::from_slice(&to_bytes(real_summary.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(real_summary["events_today"], 0);
        assert_eq!(real_summary["fingerprints"], 0);

        let reset = app
            .clone()
            .oneshot(request("POST", &format!("/api/demo/{workspace}/reset")))
            .await
            .unwrap();
        assert_eq!(reset.status(), StatusCode::OK);
        let restored = app
            .oneshot(request("GET", &format!("/api/demo/{workspace}/summary")))
            .await
            .unwrap();
        let restored: Value =
            serde_json::from_slice(&to_bytes(restored.into_body(), 64 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(restored["high_unacknowledged"], 1);
        assert_eq!(restored["events_today"], 18);
    }
}

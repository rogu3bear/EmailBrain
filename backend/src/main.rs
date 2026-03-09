use std::{
    env, fs,
    net::SocketAddr,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, Query, State},
    http::{
        HeaderValue, Method, StatusCode,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
    routing::{get, post},
};
use reqwest::Client;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{error, info, warn};

const DEFAULT_FRONTEND_ORIGIN: &str = "http://localhost:3900";
const DEFAULT_LM_STUDIO_URL: &str = "http://127.0.0.1:1234";
const DEFAULT_JKCA_APP_BASE_URL: &str = "http://127.0.0.1:8100";
const DEFAULT_INFERENCE_PROVIDER: &str = "lmstudio";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3901";
const DEFAULT_CHAT_MAX_TOKENS: i64 = 150;
const DEFAULT_PAGE_SIZE: i64 = 100;
const MAX_PAGE_SIZE: i64 = 200;
const MAX_LOG_CHARS: usize = 2_000;
const SCHEMA_SQL: &str = include_str!("../db/schema.sql");
const REQUIRED_TABLES: [&str; 4] = ["emails", "adapters", "insights", "logs"];

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    project_root: PathBuf,
    lm_studio_url: String,
    jkca_app_base_url: String,
    inference_provider: InferenceProvider,
    chat_max_tokens: i64,
    http_client: Client,
}

#[derive(Clone, Copy, Debug)]
enum InferenceProvider {
    Jkca,
    LmStudio,
}

impl InferenceProvider {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "jkca" | "jkca-agent" => Ok(Self::Jkca),
            "lmstudio" | "lm_studio" | "lm-studio" => Ok(Self::LmStudio),
            other => Err(format!(
                "unsupported EMAILBRAIN_INFERENCE_PROVIDER '{other}' (expected 'jkca' or 'lmstudio')"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Jkca => "jkca",
            Self::LmStudio => "lmstudio",
        }
    }
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "detail": self.message }))).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct EmailInput {
    subject: String,
    sender: String,
    body: String,
    date: String,
    recipients: Option<String>,
    thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct EmailRecord {
    id: i64,
    thread_id: Option<String>,
    sender: String,
    recipients: Option<String>,
    subject: String,
    date: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct EmailListItem {
    id: i64,
    subject: String,
    sender: String,
    date: String,
    recipients: Option<String>,
    thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct AdapterRecord {
    id: i64,
    name: String,
    path: String,
    train_tokens: i64,
    created_at: String,
}

#[derive(Debug)]
struct AdapterWithMetadata {
    id: i64,
    name: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    prompt: String,
    adapter_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
struct EmailListResponse {
    emails: Vec<EmailListItem>,
}

#[derive(Debug, Serialize)]
struct EmailDetailResponse {
    email: EmailRecord,
}

#[derive(Debug, Serialize)]
struct AdapterListResponse {
    adapters: Vec<AdapterRecord>,
}

#[derive(Debug, Serialize)]
struct ReceiveEmailResponse {
    message: String,
    email_id: i64,
    email: EmailInput,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "emailbrain_backend=info,tower_http=info".into()),
        )
        .init();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or("backend directory must have a parent project root")?;
    let db_path = manifest_dir.join("db").join("mail.db");
    ensure_database(&db_path)?;

    let frontend_origin =
        env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| DEFAULT_FRONTEND_ORIGIN.to_string());
    let inference_provider = match InferenceProvider::parse(
        &env::var("EMAILBRAIN_INFERENCE_PROVIDER")
            .unwrap_or_else(|_| DEFAULT_INFERENCE_PROVIDER.to_string()),
    ) {
        Ok(provider) => provider,
        Err(message) => {
            warn!("{message}; falling back to {DEFAULT_INFERENCE_PROVIDER}");
            InferenceProvider::LmStudio
        }
    };
    let chat_max_tokens = parse_chat_max_tokens(
        &env::var("EMAILBRAIN_CHAT_MAX_TOKENS")
            .unwrap_or_else(|_| DEFAULT_CHAT_MAX_TOKENS.to_string()),
    );
    let state = AppState {
        db_path,
        project_root,
        lm_studio_url: env::var("LM_STUDIO_URL").unwrap_or_else(|_| DEFAULT_LM_STUDIO_URL.into()),
        jkca_app_base_url: env::var("JKCA_APP_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_JKCA_APP_BASE_URL.into()),
        inference_provider,
        chat_max_tokens,
        http_client: Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?,
    };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(configured_frontend_origins(&frontend_origin)))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([ACCEPT, CONTENT_TYPE])
        .allow_credentials(true);
    let provider_label = state.inference_provider.as_str();

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/adapters", get(list_adapters))
        .route("/api/v1/emails", get(list_emails).post(receive_email))
        .route("/api/v1/emails/{email_id}", get(get_email))
        .route("/api/v1/chat", post(chat))
        .layer(cors)
        .with_state(state);

    let bind_addr =
        parse_bind_addr(&env::var("EMAILBRAIN_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into()));
    let addr: SocketAddr = bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|error| format!("failed to bind EmailBrain backend on http://{bind_addr}: {error}"))?;
    info!(
        "EmailBrain Rust backend listening on http://{bind_addr} using inference provider {}",
        provider_label
    );
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_adapters(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<AdapterListResponse>, AppError> {
    let conn = open_db(&state.db_path)?;
    let (limit, offset) = pagination(query);
    let mut stmt = conn
        .prepare(
            "SELECT id, name, path, train_tokens, created_at FROM adapters ORDER BY datetime(created_at) DESC, id DESC LIMIT ?1 OFFSET ?2",
        )
        .map_err(db_error)?;
    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(AdapterRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                train_tokens: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(db_error)?;

    let adapters = rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?;

    Ok(Json(AdapterListResponse { adapters }))
}

async fn list_emails(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<EmailListResponse>, AppError> {
    let conn = open_db(&state.db_path)?;
    let (limit, offset) = pagination(query);
    let mut stmt = conn
        .prepare(
            "SELECT id, subject, sender, date, recipients, thread_id FROM emails ORDER BY datetime(date) DESC, id DESC LIMIT ?1 OFFSET ?2",
        )
        .map_err(db_error)?;
    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(EmailListItem {
                id: row.get(0)?,
                subject: row.get(1)?,
                sender: row.get(2)?,
                date: row.get(3)?,
                recipients: row.get(4)?,
                thread_id: row.get(5)?,
            })
        })
        .map_err(db_error)?;

    let emails = rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?;
    Ok(Json(EmailListResponse { emails }))
}

async fn get_email(
    AxumPath(email_id): AxumPath<i64>,
    State(state): State<AppState>,
) -> Result<Json<EmailDetailResponse>, AppError> {
    if email_id < 1 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email ID must be a positive integer",
        ));
    }

    let conn = open_db(&state.db_path)?;
    let email = conn
        .query_row(
            "SELECT id, thread_id, sender, recipients, subject, date, body FROM emails WHERE id = ?1",
            [email_id],
            |row| {
                Ok(EmailRecord {
                    id: row.get(0)?,
                    thread_id: row.get(1)?,
                    sender: row.get(2)?,
                    recipients: row.get(3)?,
                    subject: row.get(4)?,
                    date: row.get(5)?,
                    body: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(db_error)?
        .ok_or_else(|| {
            AppError::new(
                StatusCode::NOT_FOUND,
                format!("Email with ID {email_id} not found"),
            )
        })?;

    Ok(Json(EmailDetailResponse { email }))
}

async fn receive_email(
    State(state): State<AppState>,
    Json(email): Json<EmailInput>,
) -> Result<Json<ReceiveEmailResponse>, AppError> {
    let email = validate_email_input(&state, email)?;
    let conn = open_db(&state.db_path)?;
    let existing_email_id = conn
        .query_row(
            "SELECT id FROM emails WHERE subject = ?1 AND sender = ?2 AND body = ?3 AND date = ?4 AND COALESCE(recipients, '') = COALESCE(?5, '') AND COALESCE(thread_id, '') = COALESCE(?6, '')",
            params![
                &email.subject,
                &email.sender,
                &email.body,
                &email.date,
                &email.recipients,
                &email.thread_id
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(db_error)?;

    if let Some(email_id) = existing_email_id {
        info!(
            "email '{}' from {} already exists as {}",
            email.subject, email.sender, email_id
        );
        return Ok(Json(ReceiveEmailResponse {
            message: "Email already exists".into(),
            email_id,
            email,
        }));
    }

    conn.execute(
        "INSERT INTO emails (subject, sender, body, date, recipients, thread_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            &email.subject,
            &email.sender,
            &email.body,
            &email.date,
            &email.recipients,
            &email.thread_id
        ],
    )
    .map_err(db_error)?;

    let email_id = conn.last_insert_rowid();
    info!("stored email '{}' from {}", email.subject, email.sender);

    Ok(Json(ReceiveEmailResponse {
        message: "Email received successfully".into(),
        email_id,
        email,
    }))
}

async fn chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<Value>, AppError> {
    if request.prompt.trim().is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Prompt must not be empty",
        ));
    }

    let adapter = match request.adapter_id {
        Some(adapter_id) => Some(fetch_adapter(&state, adapter_id)?),
        None => None,
    };
    let provider = preferred_inference_provider(state.inference_provider, adapter.is_some());

    let response_json = match provider {
        InferenceProvider::Jkca => chat_via_jkca(&state, &request, adapter.as_ref()).await?,
        InferenceProvider::LmStudio => {
            if adapter.is_some() && matches!(state.inference_provider, InferenceProvider::Jkca) {
                info!(
                    "routing adapter-backed chat through LM Studio because JKCA does not support LoRA adapters"
                );
            }
            chat_via_lm_studio(&state, &request, adapter.as_ref()).await?
        }
    };

    let tokens_out = extract_response_text(&response_json)
        .map(count_words)
        .or_else(|| {
            extract_jkca_generated_text(&response_json).map(|content| count_words(&content))
        })
        .unwrap_or(0);
    let response_excerpt = extract_response_text(&response_json)
        .map(ToOwned::to_owned)
        .or_else(|| extract_jkca_generated_text(&response_json))
        .unwrap_or_else(|| "[non-text chat response]".into());
    if let Err(error) = log_chat_interaction(
        &state,
        &request.prompt,
        &response_excerpt,
        count_words(&request.prompt),
        tokens_out,
        request.adapter_id,
    ) {
        warn!("failed to persist chat interaction: {}", error.message);
    }

    Ok(Json(response_json))
}

async fn chat_via_lm_studio(
    state: &AppState,
    request: &ChatRequest,
    adapter: Option<&AdapterWithMetadata>,
) -> Result<Value, AppError> {
    let mut payload = json!({
        "model": "local-model",
        "messages": [{ "role": "user", "content": request.prompt.trim() }],
        "temperature": 0.7,
        "max_tokens": state.chat_max_tokens
    });

    if let Some(adapter) = adapter {
        let adapter_path = resolve_adapter_path(&state.project_root, &adapter.path)?;
        if !adapter_path.exists() {
            return Err(AppError::new(
                StatusCode::NOT_FOUND,
                format!("LoRA adapter file not found at {}", adapter_path.display()),
            ));
        }
        if adapter_path.is_dir()
            && adapter_path
                .read_dir()
                .map_err(|error| {
                    AppError::internal(format!(
                        "Failed to inspect adapter directory {}: {error}",
                        adapter_path.display()
                    ))
                })?
                .next()
                .is_none()
        {
            return Err(AppError::new(
                StatusCode::BAD_REQUEST,
                format!("LoRA adapter directory is empty at {}", adapter_path.display()),
            ));
        }

        payload["lora"] = json!({
            "adapter_path": adapter_path.to_string_lossy()
        });

        info!(
            "using adapter {} ({}) via LM Studio",
            adapter.name, adapter.id
        );
    }

    let lm_response = state
        .http_client
        .post(lm_chat_url(&state.lm_studio_url))
        .json(&payload)
        .send()
        .await
        .map_err(map_lmstudio_error)?;

    let status = lm_response.status();
    let response_text = lm_response.text().await.map_err(map_lmstudio_error)?;
    if !status.is_success() {
        error!("LM Studio API error {}: {}", status, response_text);
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("LM Studio API error: {response_text}"),
        ));
    }

    serde_json::from_str(&response_text).map_err(|error| {
        AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("Failed to decode LM Studio response: {error}"),
        )
    })
}

async fn chat_via_jkca(
    state: &AppState,
    request: &ChatRequest,
    adapter: Option<&AdapterWithMetadata>,
) -> Result<Value, AppError> {
    if let Some(adapter) = adapter {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            format!(
                "JKCA shared inference does not support EmailBrain adapter selection yet (adapter '{}')",
                adapter.name
            ),
        ));
    }

    let payload = json!({
        "user_content": request.prompt.trim(),
        "max_tokens": state.chat_max_tokens
    });

    let response = state
        .http_client
        .post(jkca_runtime_generate_url(&state.jkca_app_base_url))
        .json(&payload)
        .send()
        .await
        .map_err(map_jkca_error)?;

    let status = response.status();
    let response_text = response.text().await.map_err(map_jkca_error)?;
    if !status.is_success() {
        error!("JKCA runtime API error {}: {}", status, response_text);
        return Err(AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("JKCA runtime API error: {response_text}"),
        ));
    }

    let raw_json: Value = serde_json::from_str(&response_text).map_err(|error| {
        AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("Failed to decode JKCA response: {error}"),
        )
    })?;
    let generated = extract_jkca_generated_text(&raw_json).ok_or_else(|| {
        AppError::internal("JKCA runtime response did not include generated text")
    })?;

    Ok(normalize_chat_response(
        &generated,
        "jkca-shared-runtime",
        Some(raw_json),
    ))
}

fn ensure_database(db_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(db_path)?;
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    let tables_exist: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('emails', 'adapters', 'insights', 'logs')",
        [],
        |row| row.get(0),
    )?;

    if tables_exist < REQUIRED_TABLES.len() as i64 {
        conn.execute_batch(SCHEMA_SQL)?;
    }

    seed_database(&mut conn)?;
    Ok(())
}

fn seed_database(conn: &mut Connection) -> Result<(), rusqlite::Error> {
    let email_count: i64 = conn.query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))?;
    if email_count == 0 {
        conn.execute(
            "INSERT INTO emails (subject, sender, body, date, recipients, thread_id) VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), ?4, ?5)",
            params![
                "Welcome to EmailBrain",
                "system@emailbrain.local",
                "EmailBrain is ready. Connect the Swift Mail app or POST to /api/v1/emails to ingest real messages.",
                "you@emailbrain.local",
                "seed-welcome"
            ],
        )?;
    }

    let adapter_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM adapters", [], |row| row.get(0))?;
    if adapter_count == 0 {
        conn.execute(
            "INSERT INTO adapters (name, path, train_tokens) VALUES (?1, ?2, ?3)",
            params![
                "Inbox Concierge",
                "adapters/output/inbox-concierge",
                4096i64
            ],
        )?;
    }

    Ok(())
}

fn open_db(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(db_path).map_err(db_error)?;
    conn.busy_timeout(Duration::from_secs(5)).map_err(db_error)?;
    Ok(conn)
}

fn fetch_adapter(state: &AppState, adapter_id: i64) -> Result<AdapterWithMetadata, AppError> {
    if adapter_id < 1 {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Adapter ID must be a positive integer",
        ));
    }

    let conn = open_db(&state.db_path)?;
    conn.query_row(
        "SELECT id, name, path FROM adapters WHERE id = ?1",
        [adapter_id],
        |row| {
            Ok(AdapterWithMetadata {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(db_error)?
    .ok_or_else(|| {
        AppError::new(
            StatusCode::NOT_FOUND,
            format!("LoRA adapter with ID {adapter_id} not found"),
        )
    })
}

fn log_chat_interaction(
    state: &AppState,
    prompt: &str,
    response: &str,
    tokens_in: i64,
    tokens_out: i64,
    adapter_id: Option<i64>,
) -> Result<(), AppError> {
    let conn = open_db(&state.db_path)?;
    conn.execute(
        "INSERT INTO logs (prompt, response, tokens_in, tokens_out, adapter_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            truncate_for_log(prompt),
            truncate_for_log(response),
            tokens_in,
            tokens_out,
            adapter_id
        ],
    )
    .map_err(db_error)?;
    Ok(())
}

fn extract_response_text(response: &Value) -> Option<&str> {
    response
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
}

fn extract_jkca_generated_text(response: &Value) -> Option<String> {
    response
        .pointer("/downstream/text")
        .and_then(Value::as_str)
        .or_else(|| {
            response
                .pointer("/downstream/generated_text")
                .and_then(Value::as_str)
        })
        .or_else(|| {
            response
                .pointer("/downstream/choices/0/message/content")
                .and_then(Value::as_str)
        })
        .or_else(|| response.pointer("/text").and_then(Value::as_str))
        .or_else(|| response.pointer("/generated_text").and_then(Value::as_str))
        .or_else(|| {
            response
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_chat_response(content: &str, model: &str, raw: Option<Value>) -> Value {
    let mut response = json!({
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        }]
    });

    if let Some(raw) = raw {
        response["provider_response"] = raw;
    }

    response
}

fn count_words(content: &str) -> i64 {
    content.split_whitespace().count() as i64
}

fn preferred_inference_provider(
    default_provider: InferenceProvider,
    has_adapter: bool,
) -> InferenceProvider {
    if has_adapter {
        InferenceProvider::LmStudio
    } else {
        default_provider
    }
}

fn lm_chat_url(base_url: &str) -> String {
    format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
}

fn jkca_runtime_generate_url(base_url: &str) -> String {
    format!("{}/api/runtime/generate", base_url.trim_end_matches('/'))
}

fn db_error(error: rusqlite::Error) -> AppError {
    AppError::internal(format!("Database error occurred: {error}"))
}

fn map_lmstudio_error(error: reqwest::Error) -> AppError {
    if error.is_connect() {
        return AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Unable to connect to LM Studio server. Please ensure it's running on http://127.0.0.1:1234",
        );
    }

    if error.is_timeout() {
        return AppError::new(
            StatusCode::GATEWAY_TIMEOUT,
            "Request to LM Studio server timed out",
        );
    }

    if error.is_builder() || error.is_request() {
        return AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("Invalid LM Studio request: {error}"),
        );
    }

    AppError::internal(format!("Unexpected HTTP error: {error}"))
}

fn map_jkca_error(error: reqwest::Error) -> AppError {
    if error.is_connect() {
        return AppError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "Unable to connect to JKCA runtime. Please ensure jkca-agent is running on http://127.0.0.1:8100",
        );
    }

    if error.is_timeout() {
        return AppError::new(
            StatusCode::GATEWAY_TIMEOUT,
            "Request to JKCA runtime timed out",
        );
    }

    if error.is_builder() || error.is_request() {
        return AppError::new(
            StatusCode::BAD_GATEWAY,
            format!("Invalid JKCA request: {error}"),
        );
    }

    AppError::internal(format!("Unexpected JKCA HTTP error: {error}"))
}

fn validate_email_input(state: &AppState, email: EmailInput) -> Result<EmailInput, AppError> {
    let subject = email.subject.trim();
    if subject.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email subject must not be empty",
        ));
    }

    let sender = email.sender.trim();
    if sender.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email sender must not be empty",
        ));
    }

    let body = email.body.trim();
    if body.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email body must not be empty",
        ));
    }

    let normalized_date = normalize_timestamp(&state.db_path, &email.date)?;

    Ok(EmailInput {
        subject: subject.to_string(),
        sender: sender.to_string(),
        body: body.to_string(),
        date: normalized_date,
        recipients: email.recipients.and_then(trim_optional),
        thread_id: email.thread_id.and_then(trim_optional),
    })
}

fn normalize_timestamp(db_path: &Path, raw: &str) -> Result<String, AppError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            "Email date must not be empty",
        ));
    }

    let conn = open_db(db_path)?;
    conn.query_row(
        "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', ?1)",
        [value],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(db_error)?
    .filter(|normalized| !normalized.is_empty())
    .ok_or_else(|| {
        AppError::new(
            StatusCode::BAD_REQUEST,
            format!("Email date '{value}' is not a supported timestamp"),
        )
    })
}

fn trim_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn pagination(query: ListQuery) -> (i64, i64) {
    let limit = query.limit.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    (limit, offset)
}

fn configured_frontend_origins(raw: &str) -> Vec<HeaderValue> {
    let mut values = Vec::new();
    for origin in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        push_origin_aliases(&mut values, origin);
    }

    if values.is_empty() {
        push_origin_aliases(&mut values, DEFAULT_FRONTEND_ORIGIN);
    }

    values
}

fn push_origin_aliases(values: &mut Vec<HeaderValue>, origin: &str) {
    for candidate in local_origin_aliases(origin) {
        if let Ok(header) = candidate.parse::<HeaderValue>() {
            if !values.iter().any(|existing| existing == &header) {
                values.push(header);
            }
        } else {
            warn!("ignoring invalid FRONTEND_ORIGIN entry '{candidate}'");
        }
    }
}

fn local_origin_aliases(origin: &str) -> Vec<String> {
    let mut aliases = vec![origin.to_string()];
    if let Some(rest) = origin.strip_prefix("http://localhost:") {
        aliases.push(format!("http://127.0.0.1:{rest}"));
    } else if let Some(rest) = origin.strip_prefix("http://127.0.0.1:") {
        aliases.push(format!("http://localhost:{rest}"));
    } else if let Some(rest) = origin.strip_prefix("https://localhost:") {
        aliases.push(format!("https://127.0.0.1:{rest}"));
    } else if let Some(rest) = origin.strip_prefix("https://127.0.0.1:") {
        aliases.push(format!("https://localhost:{rest}"));
    }
    aliases
}

fn parse_bind_addr(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.parse::<SocketAddr>().is_ok() {
        return trimmed.to_string();
    }

    warn!(
        "invalid EMAILBRAIN_BIND '{}'; falling back to {}",
        trimmed, DEFAULT_BIND_ADDR
    );
    DEFAULT_BIND_ADDR.to_string()
}

fn parse_chat_max_tokens(raw: &str) -> i64 {
    match raw.trim().parse::<i64>() {
        Ok(value) if value > 0 => value,
        _ => {
            warn!(
                "invalid EMAILBRAIN_CHAT_MAX_TOKENS '{}'; falling back to {}",
                raw, DEFAULT_CHAT_MAX_TOKENS
            );
            DEFAULT_CHAT_MAX_TOKENS
        }
    }
}

fn resolve_adapter_path(project_root: &Path, adapter_path: &str) -> Result<PathBuf, AppError> {
    let relative_path = Path::new(adapter_path);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::new(
            StatusCode::BAD_REQUEST,
            format!("Adapter path '{}' must stay inside the project root", adapter_path),
        ));
    }

    Ok(project_root.join(relative_path))
}

fn truncate_for_log(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_LOG_CHARS {
        return trimmed.to_string();
    }

    let mut truncated = trimmed.chars().take(MAX_LOG_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_BIND_ADDR, DEFAULT_CHAT_MAX_TOKENS, InferenceProvider, configured_frontend_origins,
        count_words, extract_jkca_generated_text, jkca_runtime_generate_url, lm_chat_url,
        parse_bind_addr, parse_chat_max_tokens, preferred_inference_provider, resolve_adapter_path,
        truncate_for_log,
    };
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn trims_trailing_slash_when_building_lm_studio_url() {
        assert_eq!(
            lm_chat_url("http://127.0.0.1:1234/"),
            "http://127.0.0.1:1234/v1/chat/completions"
        );
    }

    #[test]
    fn trims_trailing_slash_when_building_jkca_runtime_url() {
        assert_eq!(
            jkca_runtime_generate_url("http://127.0.0.1:8100/"),
            "http://127.0.0.1:8100/api/runtime/generate"
        );
    }

    #[test]
    fn counts_words_from_whitespace_boundaries() {
        assert_eq!(count_words("one   two\nthree"), 3);
    }

    #[test]
    fn extracts_generated_text_from_jkca_proxy_shape() {
        let payload = json!({
            "downstream": {
                "text": "shared response"
            }
        });

        assert_eq!(
            extract_jkca_generated_text(&payload).as_deref(),
            Some("shared response")
        );
    }

    #[test]
    fn prefers_lm_studio_when_adapter_is_selected() {
        assert!(matches!(
            preferred_inference_provider(InferenceProvider::Jkca, true),
            InferenceProvider::LmStudio
        ));
    }

    #[test]
    fn preserves_default_provider_for_base_chat() {
        assert!(matches!(
            preferred_inference_provider(InferenceProvider::Jkca, false),
            InferenceProvider::Jkca
        ));
    }

    #[test]
    fn adds_localhost_alias_for_default_frontend_origin() {
        let origins = configured_frontend_origins("http://localhost:3900");
        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn falls_back_to_default_bind_addr_when_invalid() {
        assert_eq!(parse_bind_addr("not-an-addr"), DEFAULT_BIND_ADDR);
    }

    #[test]
    fn falls_back_to_default_chat_max_tokens_when_invalid() {
        assert_eq!(parse_chat_max_tokens("oops"), DEFAULT_CHAT_MAX_TOKENS);
    }

    #[test]
    fn rejects_adapter_paths_that_escape_project_root() {
        let result = resolve_adapter_path(Path::new("/tmp/project"), "../escape");
        assert!(result.is_err());
    }

    #[test]
    fn truncates_large_log_entries() {
        let value = "x".repeat(2_100);
        let truncated = truncate_for_log(&value);
        assert!(truncated.len() < value.len());
        assert!(truncated.ends_with("..."));
    }
}

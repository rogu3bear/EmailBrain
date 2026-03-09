use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
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
use tracing::{error, info};

const DEFAULT_FRONTEND_ORIGIN: &str = "http://localhost:3900";
const DEFAULT_LM_STUDIO_URL: &str = "http://127.0.0.1:1234";
const DEFAULT_JKCA_APP_BASE_URL: &str = "http://127.0.0.1:8100";
const DEFAULT_INFERENCE_PROVIDER: &str = "lmstudio";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:3901";
const SEED_EMAIL_DATE: &str = "2026-03-09T00:00:00Z";

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    project_root: PathBuf,
    lm_studio_url: String,
    jkca_app_base_url: String,
    inference_provider: InferenceProvider,
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
    let schema_path = manifest_dir.join("db").join("schema.sql");
    ensure_database(&db_path, &schema_path)?;

    let frontend_origin =
        env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| DEFAULT_FRONTEND_ORIGIN.to_string());
    let inference_provider = InferenceProvider::parse(
        &env::var("EMAILBRAIN_INFERENCE_PROVIDER")
            .unwrap_or_else(|_| DEFAULT_INFERENCE_PROVIDER.to_string()),
    )?;
    let state = AppState {
        db_path,
        project_root,
        lm_studio_url: env::var("LM_STUDIO_URL").unwrap_or_else(|_| DEFAULT_LM_STUDIO_URL.into()),
        jkca_app_base_url: env::var("JKCA_APP_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_JKCA_APP_BASE_URL.into()),
        inference_provider,
        http_client: Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?,
    };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(frontend_origin.parse::<HeaderValue>()?))
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

    let bind_addr = env::var("EMAILBRAIN_BIND").unwrap_or_else(|_| DEFAULT_BIND_ADDR.into());
    let addr: SocketAddr = bind_addr.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
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
) -> Result<Json<AdapterListResponse>, AppError> {
    let conn = open_db(&state.db_path)?;
    let mut stmt = conn
        .prepare("SELECT id, name, path, train_tokens, created_at FROM adapters ORDER BY created_at DESC")
        .map_err(db_error)?;
    let rows = stmt
        .query_map([], |row| {
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

async fn list_emails(State(state): State<AppState>) -> Result<Json<EmailListResponse>, AppError> {
    let conn = open_db(&state.db_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT id, subject, sender, date, recipients, thread_id FROM emails ORDER BY date DESC",
        )
        .map_err(db_error)?;
    let rows = stmt
        .query_map([], |row| {
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
        .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, format!("Email with ID {email_id} not found")))?;

    Ok(Json(EmailDetailResponse { email }))
}

async fn receive_email(
    State(state): State<AppState>,
    Json(email): Json<EmailInput>,
) -> Result<Json<ReceiveEmailResponse>, AppError> {
    let conn = open_db(&state.db_path)?;
    conn.execute(
        "INSERT INTO emails (subject, sender, body, date, recipients, thread_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            email.subject,
            email.sender,
            email.body,
            email.date,
            email.recipients,
            email.thread_id
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
    log_chat_interaction(
        &state,
        &request.prompt,
        &response_json.to_string(),
        count_words(&request.prompt),
        tokens_out as i64,
        request.adapter_id,
    )?;

    Ok(Json(response_json))
}

async fn chat_via_lm_studio(
    state: &AppState,
    request: &ChatRequest,
    adapter: Option<&AdapterWithMetadata>,
) -> Result<Value, AppError> {
    let mut payload = json!({
        "model": "local-model",
        "messages": [{ "role": "user", "content": request.prompt }],
        "temperature": 0.7,
        "max_tokens": 150
    });

    if let Some(adapter) = adapter {
        let adapter_path = state.project_root.join(&adapter.path);
        if !adapter_path.exists() {
            return Err(AppError::new(
                StatusCode::NOT_FOUND,
                format!("LoRA adapter file not found at {}", adapter_path.display()),
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
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            format!("LM Studio API error: {response_text}"),
        ));
    }

    serde_json::from_str(&response_text).map_err(|error| {
        AppError::internal(format!("Failed to decode LM Studio response: {error}"))
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
        "user_content": request.prompt,
        "max_tokens": 32
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
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY),
            format!("JKCA runtime API error: {response_text}"),
        ));
    }

    let raw_json: Value = serde_json::from_str(&response_text)
        .map_err(|error| AppError::internal(format!("Failed to decode JKCA response: {error}")))?;
    let generated = extract_jkca_generated_text(&raw_json).ok_or_else(|| {
        AppError::internal("JKCA runtime response did not include generated text")
    })?;

    Ok(normalize_chat_response(
        &generated,
        "jkca-shared-runtime",
        Some(raw_json),
    ))
}

fn ensure_database(db_path: &Path, schema_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if !schema_path.exists() {
        return Err(format!("Schema file not found at {}", schema_path.display()).into());
    }

    let mut conn = Connection::open(db_path)?;
    let tables_exist: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('emails', 'adapters', 'logs')",
        [],
        |row| row.get(0),
    )?;

    if tables_exist == 0 {
        let schema_sql = fs::read_to_string(schema_path)?;
        conn.execute_batch(&schema_sql)?;
    }

    seed_database(&mut conn)?;
    Ok(())
}

fn seed_database(conn: &mut Connection) -> Result<(), rusqlite::Error> {
    let email_count: i64 = conn.query_row("SELECT COUNT(*) FROM emails", [], |row| row.get(0))?;
    if email_count == 0 {
        conn.execute(
            "INSERT INTO emails (subject, sender, body, date, recipients, thread_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "Welcome to EmailBrain",
                "system@emailbrain.local",
                "EmailBrain is ready. Connect the Swift Mail app or POST to /api/v1/emails to ingest real messages.",
                SEED_EMAIL_DATE,
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
    Connection::open(db_path).map_err(db_error)
}

fn fetch_adapter(state: &AppState, adapter_id: i64) -> Result<AdapterWithMetadata, AppError> {
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
        params![prompt, response, tokens_in, tokens_out, adapter_id],
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

    AppError::internal(format!("Unexpected JKCA HTTP error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        InferenceProvider, count_words, extract_jkca_generated_text, jkca_runtime_generate_url,
        lm_chat_url, preferred_inference_provider,
    };
    use serde_json::json;

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
}

use axum::{
    Json, Router,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use study_domain::*;
use study_engine::{
    Store,
    connectors::{Config, Github, Notion},
    generation::OpenAi,
};
use subtle::ConstantTimeEq;
use tower_http::cors::CorsLayer;
#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub token_hash: String,
    pub config: PathBuf,
}
type Error = (StatusCode, String);
fn err(e: impl std::fmt::Display) -> Error {
    eprintln!("study-server: {e}");
    (
        StatusCode::BAD_REQUEST,
        "Opération refusée ou incomplète ; consulter le journal privé du serveur.".into(),
    )
}
async fn auth(State(s): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if hash(token)
        .as_bytes()
        .ct_eq(s.token_hash.as_bytes())
        .unwrap_u8()
        != 1
    {
        return (StatusCode::UNAUTHORIZED, "Authentification requise").into_response();
    }
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
}
pub fn router(state: AppState, origin: &str) -> anyhow::Result<Router> {
    let cors = CorsLayer::new()
        .allow_origin(origin.parse::<HeaderValue>()?)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);
    let state = Arc::new(state);
    Ok(Router::new()
        .route("/snapshot", get(snapshot))
        .route("/sync", post(sync))
        .route("/generate", post(generate))
        .route("/review", post(review))
        .route("/validate", post(validate))
        .route("/document", put(document))
        .route("/question", put(question))
        .route("/fiche", put(fiche))
        .route("/report", post(report))
        .route("/pdf/{id}", get(pdf))
        .route("/original/{id}", get(original))
        .route("/publish/github", post(publish_github))
        .route("/publish/notion/{id}", post(publish_notion))
        .route("/candidates", get(candidates))
        .route("/candidate", post(candidate))
        .layer(axum::extract::DefaultBodyLimit::max(28 * 1024 * 1024))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth))
        .layer(cors)
        .with_state(state))
}
async fn snapshot(State(s): State<Arc<AppState>>) -> Result<Json<Snapshot>, Error> {
    Ok(Json(s.store.snapshot().map_err(err)?))
}
async fn sync(State(s): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, Error> {
    let cfg = Config::load(&s.config).map_err(err)?;
    let n = Notion::from_env().map_err(err)?;
    let r =
        study_engine::pipeline::sync_notion(&s.store, &OpenAi::from_env().map_err(err)?, &n, &cfg)
            .await
            .map_err(err)?;
    Ok(Json(serde_json::to_value(r).map_err(err)?))
}
async fn generate(State(s): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, Error> {
    let r = study_engine::pipeline::sync(
        &s.store,
        &OpenAi::from_env().map_err(err)?,
        s.store.list("document").map_err(err)?,
    )
    .await
    .map_err(err)?;
    Ok(Json(serde_json::to_value(r).map_err(err)?))
}
#[derive(Deserialize)]
struct Review {
    question_id: String,
    quality: u8,
}
async fn review(
    State(s): State<Arc<AppState>>,
    Json(r): Json<Review>,
) -> Result<Json<Progress>, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let q = s
        .store
        .get::<Question>("question", &r.question_id)
        .map_err(err)?
        .ok_or_else(|| err("Question absente"))?;
    if q.status != Status::Accepted {
        return Err(err("Question non validée"));
    }
    s.store.validate_sources(&q.sources).map_err(err)?;
    let mut p = s
        .store
        .get::<Progress>("progress", &r.question_id)
        .map_err(err)?
        .unwrap_or_else(|| Progress::new(r.question_id.clone()));
    // The domain implements SM-2 for BOTH clients. No client-side grading authority.
    p.review(r.quality, chrono_now()).map_err(err)?;
    s.store
        .put("progress", &r.question_id, &p, true)
        .map_err(err)?;
    Ok(Json(p))
}
fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
#[derive(Deserialize)]
struct Validation {
    kind: String,
    id: String,
    status: Status,
}
async fn validate(
    State(s): State<Arc<AppState>>,
    Json(v): Json<Validation>,
) -> Result<StatusCode, Error> {
    s.store.accept(&v.kind, &v.id, v.status).map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn document(
    State(s): State<Arc<AppState>>,
    Json(mut d): Json<Document>,
) -> Result<StatusCode, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let old = s
        .store
        .get::<Document>("document", &d.id)
        .map_err(err)?
        .ok_or_else(|| err("Document absent ; utiliser import"))?;
    // Original bytes and generated provenance cannot be replaced through editing.
    d.transcription = old.transcription;
    d.original = old.original;
    d.content_hash = old.content_hash;
    if old.provenance_reason == "generated" {
        d.level = Some(2);
        d.provenance_reason = "generated".into();
    }
    d.status = Status::Pending;
    d.modified_at = now();
    d.validate().map_err(err)?;
    s.store.put("document", &d.id, &d, true).map_err(err)?;
    invalidate(&s.store, &d.id).map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
fn invalidate(store: &Store, id: &str) -> anyhow::Result<()> {
    for mut q in store.list::<Question>("question")? {
        if q.sources.iter().any(|c| c.document_id == id) {
            q.status = Status::Stale;
            Store::put_in(&store.db()?, "question", &q.id, &q, false, true)?;
        }
    }
    for mut f in store.list::<Fiche>("fiche")? {
        if f.sources.iter().any(|c| c.document_id == id) {
            f.status = Status::Stale;
            Store::put_in(&store.db()?, "fiche", &f.id, &f, false, true)?;
        }
    }
    Ok(())
}
async fn question(
    State(s): State<Arc<AppState>>,
    Json(mut q): Json<Question>,
) -> Result<StatusCode, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let old = s
        .store
        .get::<Question>("question", &q.id)
        .map_err(err)?
        .ok_or_else(|| err("Question absente"))?;
    q.sources = old.sources;
    q.created_at = old.created_at;
    q.updated_at = now();
    q.status = Status::Pending;
    if q.prompt.trim().is_empty() || q.answer.trim().is_empty() {
        return Err(err("Question vide"));
    }
    s.store.put("question", &q.id, &q, true).map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn fiche(
    State(s): State<Arc<AppState>>,
    Json(mut f): Json<Fiche>,
) -> Result<StatusCode, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let old = s
        .store
        .get::<Fiche>("fiche", &f.id)
        .map_err(err)?
        .ok_or_else(|| err("Fiche absente"))?;
    f.sources = old.sources;
    f.level = 2;
    f.status = Status::Pending;
    s.store.put("fiche", &f.id, &f, true).map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
#[derive(Deserialize)]
struct Issue {
    question_id: String,
    detail: String,
}
async fn report(State(s): State<Arc<AppState>>, Json(i): Json<Issue>) -> Result<StatusCode, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let mut q = s
        .store
        .get::<Question>("question", &i.question_id)
        .map_err(err)?
        .ok_or_else(|| err("Question absente"))?;
    q.status = Status::Pending;
    s.store.put("question", &q.id, &q, true).map_err(err)?;
    let key = id(&[&i.question_id, &now()]);
    s.store
        .put(
            "issue",
            &key,
            &serde_json::json!({"id":key,"question_id":i.question_id,"detail":i.detail}),
            true,
        )
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn pdf(State(s): State<Arc<AppState>>, Path(id): Path<String>) -> Result<Response, Error> {
    let f = s
        .store
        .get::<Fiche>("fiche", &id)
        .map_err(err)?
        .ok_or_else(|| err("Fiche absente"))?;
    let root = s.store.root.join("exports");
    let path = tokio::task::spawn_blocking(move || study_engine::export::export(&f, &root, true))
        .await
        .map_err(err)?
        .map_err(err)?;
    let bytes = tokio::fs::read(path.join("fiche.pdf")).await.map_err(err)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=fiche.pdf",
            ),
        ],
        bytes,
    )
        .into_response())
}
async fn publish_github(State(s): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let c = Config::load(&s.config).map_err(err)?;
    let url = Github::from_env(
        c.github_repository
            .as_deref()
            .ok_or_else(|| err("Dépôt privé dédié requis"))?,
    )
    .map_err(err)?
    .publish(&s.store)
    .await
    .map_err(err)?;
    Ok(Json(serde_json::json!({"url":url})))
}
async fn publish_notion(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Error> {
    let _guard = s.store.lock().map_err(err)?;
    let c = Config::load(&s.config).map_err(err)?;
    let f = s
        .store
        .get::<Fiche>("fiche", &id)
        .map_err(err)?
        .ok_or_else(|| err("Fiche absente"))?;
    let url = Notion::from_env()
        .map_err(err)?
        .publish_fiche(&s.store, &c, &f)
        .await
        .map_err(err)?;
    Ok(Json(serde_json::json!({"url":url})))
}
async fn candidates(State(s): State<Arc<AppState>>) -> Result<Json<serde_json::Value>, Error> {
    Ok(Json(
        serde_json::json!({"documents":s.store.list::<Document>("candidate-document").map_err(err)?,"fiches":s.store.list::<Fiche>("candidate-fiche").map_err(err)?,"questions":s.store.list::<Question>("candidate-question").map_err(err)?}),
    ))
}
#[derive(Deserialize)]
struct CandidateChoice {
    kind: String,
    id: String,
    apply: bool,
}
async fn candidate(
    State(s): State<Arc<AppState>>,
    Json(c): Json<CandidateChoice>,
) -> Result<StatusCode, Error> {
    s.store
        .apply_candidate(&c.kind, &c.id, c.apply)
        .map_err(err)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn original(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Response, Error> {
    let d = s
        .store
        .get::<Document>("document", &id)
        .map_err(err)?
        .ok_or_else(|| err("Document absent"))?;
    let path = if d.original.starts_with("https://www.notion.so/") {
        s.store
            .root
            .join("originals")
            .join(&d.content_hash)
            .join(format!("{}.notion.json", d.id))
    } else {
        std::path::PathBuf::from(&d.original)
    };
    let canonical = path.canonicalize().map_err(err)?;
    let allowed = s.store.root.join("originals").canonicalize().map_err(err)?;
    if !canonical.starts_with(allowed) {
        return Err(err("Original hors de l’archive privée"));
    }
    let bytes = tokio::fs::read(&canonical).await.map_err(err)?;
    let ext = canonical
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    let mime = match ext {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        "txt" | "md" => "text/plain; charset=utf-8",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    };
    let disposition =
        HeaderValue::from_str(&format!("attachment; filename=original.{ext}")).map_err(err)?;
    Ok((
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(mime)),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    )
        .into_response())
}

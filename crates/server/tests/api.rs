use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use study_domain::*;
use study_engine::Store;
use study_server::{AppState, router};
use tower::ServiceExt;
fn app() -> (std::path::PathBuf, Store, axum::Router) {
    let p = std::env::temp_dir().join(format!("study-api-{}", id(&[&now()])));
    let store = Store::open(&p).unwrap();
    let app = router(
        AppState {
            store: store.clone(),
            token_hash: hash("abcdefghijklmnopqrstuvwxyz0123456789"),
            config: p.join("absent.json"),
        },
        "https://cinnamonvii.github.io",
    )
    .unwrap();
    (p, store, app)
}
#[tokio::test]
async fn unauthenticated_requests_cannot_read_private_data() {
    let (_p, s, app) = app();
    let d: Vec<Document> =
        serde_json::from_str(include_str!("../../../fixtures/demo/documents.json")).unwrap();
    s.put("document", &d[0].id, &d[0], true).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn private_responses_are_not_cacheable() {
    let (_p, _s, app) = app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/snapshot")
                .header(
                    "Authorization",
                    "Bearer abcdefghijklmnopqrstuvwxyz0123456789",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store, private");
}
#[tokio::test]
async fn unapproved_origin_is_not_granted_cors_access() {
    let (_p, _s, app) = app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/snapshot")
                .header("Origin", "https://attacker.invalid")
                .header(
                    "Authorization",
                    "Bearer abcdefghijklmnopqrstuvwxyz0123456789",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|x| x.to_str().ok()),
        Some("https://attacker.invalid")
    );
}
#[tokio::test]
async fn clients_receive_identical_versioned_questions() {
    let (_p, s, app) = app();
    let fixture: Snapshot =
        serde_json::from_str(include_str!("../../../fixtures/demo/snapshot.json")).unwrap();
    for q in &fixture.questions {
        s.put("question", &q.id, q, false).unwrap();
    }
    let fetch = || {
        Request::builder()
            .uri("/snapshot")
            .header(
                "Authorization",
                "Bearer abcdefghijklmnopqrstuvwxyz0123456789",
            )
            .body(Body::empty())
            .unwrap()
    };
    let a = app.clone().oneshot(fetch()).await.unwrap();
    let b = app.oneshot(fetch()).await.unwrap();
    let a = axum::body::to_bytes(a.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let b = axum::body::to_bytes(b.into_body(), 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(a, b);
    let parsed: Snapshot = serde_json::from_slice(&a).unwrap();
    assert_eq!(parsed.questions.len(), 4);
    assert_eq!(parsed.schema_version, "1.0");
}
#[tokio::test]
async fn nonvalidated_questions_cannot_change_progress() {
    let (_p, s, app) = app();
    let mut fixture: Snapshot =
        serde_json::from_str(include_str!("../../../fixtures/demo/snapshot.json")).unwrap();
    let mut q = fixture.questions.remove(0);
    q.status = Status::Pending;
    s.put("question", &q.id, &q, true).unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/review")
                .header(
                    "Authorization",
                    "Bearer abcdefghijklmnopqrstuvwxyz0123456789",
                )
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"question_id":q.id,"quality":5}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(s.list::<Progress>("progress").unwrap().is_empty());
}

use anyhow::{Result, bail};
#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("STUDY_TOKEN")?;
    if token.len() < 32 {
        bail!("STUDY_TOKEN : minimum 32 caractères aléatoires")
    }
    let store =
        study_engine::Store::open(std::env::var("STUDY_STATE").unwrap_or_else(|_| "state".into()))?;
    let app = study_server::router(
        study_server::AppState {
            store,
            token_hash: study_domain::hash(token),
            config: std::env::var("STUDY_CONFIG")
                .unwrap_or_else(|_| "config.json".into())
                .into(),
        },
        &std::env::var("STUDY_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
    )?;
    let bind = std::env::var("STUDY_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    eprintln!("Serveur privé : {bind}");
    axum::serve(listener, app).await?;
    Ok(())
}

use anyhow::{Context, Result, bail};
use study_domain::*;
use study_engine::{
    Store,
    connectors::{Config, Github, Notion},
    generation::{FixtureGenerator, OpenAi},
};
#[tokio::main]
async fn main() -> Result<()> {
    let a = std::env::args().skip(1).collect::<Vec<_>>();
    let cmd = a.first().map(String::as_str).unwrap_or("help");
    let root = std::env::var("STUDY_STATE").unwrap_or_else(|_| "state".into());
    if cmd == "schema" {
        std::fs::create_dir_all("schemas")?;
        std::fs::write(
            "schemas/snapshot-v1.json",
            serde_json::to_string_pretty(&schemars::schema_for!(Snapshot))?,
        )?;
        return Ok(());
    }
    let s = Store::open(root)?;
    match cmd {
        "demo" => {
            if s.list::<Document>("document")?
                .iter()
                .any(|d| !d.id.starts_with("demo-"))
            {
                bail!(
                    "Utilisez un STUDY_STATE dédié : demo ne doit jamais exporter de données réelles"
                )
            };
            let docs: Vec<Document> =
                serde_json::from_str(include_str!("../../../fixtures/demo/documents.json"))?;
            let r = study_engine::pipeline::sync(&s, &FixtureGenerator, docs).await?;
            for mut d in s.list::<Document>("document")? {
                d.status = Status::Accepted;
                s.put("document", &d.id, &d, true)?;
            }
            for f in s.list::<Fiche>("fiche")? {
                s.accept("fiche", &f.id, Status::Accepted)?;
            }
            for q in s.list::<Question>("question")? {
                s.accept("question", &q.id, Status::Accepted)?;
            }
            std::fs::create_dir_all("fixtures/demo")?;
            std::fs::write(
                "fixtures/demo/snapshot.json",
                serde_json::to_vec_pretty(&s.snapshot()?)?,
            )?;
            println!("{} document(s) fictif(s) traité(s)", r.changed);
        }
        "daily" => {
            let cfg = Config::load(std::path::Path::new(
                a.get(1).context("Configuration requise")?,
            ))?;
            let n = Notion::from_env()?;
            let r = study_engine::pipeline::sync_notion(&s, &OpenAi::from_env()?, &n, &cfg).await?;
            let _guard = s.lock()?;
            for f in s
                .list::<Fiche>("fiche")?
                .into_iter()
                .filter(|f| f.status == Status::Accepted)
            {
                study_engine::export::export(&f, &s.root.join("exports"), true)?;
                if cfg
                    .notion_destinations
                    .contains_key(&format!("{}/{}", f.subject, f.chapter))
                {
                    let url = n.publish_fiche(&s, &cfg, &f).await?;
                    println!("Notion : {url}");
                }
            }
            if let Some(repo) = &cfg.github_repository {
                println!("GitHub : {}", Github::from_env(repo)?.publish(&s).await?);
            }
            println!("{}", serde_json::to_string(&r)?);
        }
        "sync" => {
            let config = Config::load(std::path::Path::new(
                a.get(1).map(String::as_str).unwrap_or("config.json"),
            ))?;
            let notion = Notion::from_env()?;
            let r = study_engine::pipeline::sync_notion(&s, &OpenAi::from_env()?, &notion, &config)
                .await?;
            println!("{}", serde_json::to_string(&r)?);
        }
        "import" => {
            if a.len() < 7 {
                bail!("import FICHIER MATIÈRE CHAPITRE TYPE NIVEAU JUSTIFICATION")
            }
            let kind: DocType = serde_json::from_value(serde_json::json!(a[4]))?;
            let level = if a[5] == "unknown" {
                None
            } else {
                Some(a[5].parse()?)
            };
            let d = study_engine::ingestion::import(
                &s,
                std::path::Path::new(&a[1]),
                &a[2],
                &a[3],
                kind,
                level,
                &a[6],
                None,
            )?;
            s.put("document", &d.id, &d, true)?;
            println!("{}", d.id);
        }
        "generate" => {
            let r =
                study_engine::pipeline::sync(&s, &OpenAi::from_env()?, s.list("document")?).await?;
            println!("{}", serde_json::to_string(&r)?);
        }
        "export" => {
            for f in s.list::<Fiche>("fiche")? {
                let p = study_engine::export::export(&f, &s.root.join("exports"), true)?;
                println!("{}", p.display());
            }
        }
        "publish" => {
            let cfg = Config::load(std::path::Path::new(
                a.get(1).context("Chemin configuration requis")?,
            ))?;
            let _guard = s.lock()?;
            let repo = cfg.github_repository.context("Dépôt privé dédié requis")?;
            println!("{}", Github::from_env(&repo)?.publish(&s).await?);
        }
        "snapshot" => println!("{}", serde_json::to_string_pretty(&s.snapshot()?)?),
        _ => println!(
            "study-engine : daily CONFIG | demo | schema | import | generate | sync CONFIG | export | publish CONFIG | snapshot\nVoir README.md ; demo utilise uniquement des fixtures."
        ),
    }
    Ok(())
}

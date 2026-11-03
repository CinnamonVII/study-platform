use crate::Store;
use anyhow::{Context, Result, bail};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use study_domain::*;
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub notion_sources: Vec<NotionSource>,
    pub notion_destinations: std::collections::BTreeMap<String, String>,
    pub github_repository: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct NotionSource {
    pub page_id: String,
    pub subject: String,
    pub chapter: String,
    pub kind: DocType,
    pub level: Option<u8>,
    #[serde(default)]
    pub references: Vec<Citation>,
    pub reason: String,
}
impl Config {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        Ok(serde_json::from_slice(&std::fs::read(path)?)?)
    }
}
pub struct Notion {
    client: reqwest::Client,
    token: String,
    base: String,
}
impl Notion {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            token: std::env::var("NOTION_TOKEN")?,
            base: "https://api.notion.com/v1".into(),
        })
    }
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut request = self
            .client
            .request(method, format!("{}/{path}", self.base))
            .bearer_auth(&self.token)
            .header("Notion-Version", "2025-09-03");
        if let Some(b) = body {
            request = request.json(&b)
        }
        let r = request.send().await?;
        if !r.status().is_success() {
            bail!(
                "Notion HTTP {} ; aucun état de succès enregistré",
                r.status()
            )
        }
        Ok(r.json().await?)
    }
    pub async fn children(&self, id: &str) -> Result<Vec<Value>> {
        let mut result = vec![];
        let mut cursor = String::new();
        loop {
            let path = format!(
                "blocks/{id}/children?page_size=100{}",
                if cursor.is_empty() {
                    String::new()
                } else {
                    format!("&start_cursor={cursor}")
                }
            );
            let v = self.request(reqwest::Method::GET, &path, None).await?;
            result.extend(
                v["results"]
                    .as_array()
                    .context("Notion : liste de blocs absente")?
                    .clone(),
            );
            if v["has_more"] != true {
                break;
            }
            cursor = v["next_cursor"]
                .as_str()
                .context("Curseur manquant")?
                .into();
        }
        Ok(result)
    }
    pub async fn scan(&self, store: &Store, config: &Config) -> Result<Vec<Document>> {
        let mut docs = vec![];
        for source in &config.notion_sources {
            let p = self
                .request(
                    reqwest::Method::GET,
                    &format!("pages/{}", source.page_id),
                    None,
                )
                .await?;
            let mut d = Document {
                id: source.page_id.clone(),
                title: source.chapter.clone(),
                kind: source.kind,
                level: source.level,
                provenance_reason: source.reason.clone(),
                subject: source.subject.clone(),
                chapter: source.chapter.clone(),
                modified_at: p["last_edited_time"].as_str().unwrap_or("").into(),
                content_hash: String::new(),
                status: Status::Pending,
                sources: source.references.clone(),
                text: String::new(),
                original: format!("https://www.notion.so/{}", source.page_id.replace('-', "")),
                private: true,
                deleted: p["archived"] == true || p["in_trash"] == true,
                uncertainties: vec![],
                transcription: None,
            };
            if d.deleted {
                if let Some(mut old) = store.get::<Document>("document", &d.id)? {
                    old.deleted = true;
                    docs.push(old);
                } else {
                    docs.push(d);
                }
                continue;
            }
            let mut queue = vec![source.page_id.clone()];
            let mut attachments = vec![];
            let mut seen = std::collections::HashSet::new();
            while let Some(block) = queue.pop() {
                if !seen.insert(block.clone()) {
                    continue;
                }
                for b in self.children(&block).await? {
                    let kind = b["type"].as_str().unwrap_or("");
                    // Child pages require their own explicit allowlist entry and classification.
                    if kind == "child_page" || kind == "child_database" {
                        continue;
                    }
                    let value = &b[kind];
                    if let Some(r) = value["rich_text"].as_array() {
                        for t in r {
                            if let Some(text) = t["plain_text"].as_str() {
                                d.text.push_str(text)
                            } else if let Some(text) = t["text"]["content"].as_str() {
                                d.text.push_str(text)
                            }
                        }
                        d.text.push('\n');
                    }
                    if kind == "equation" {
                        d.text.push_str(value["expression"].as_str().unwrap_or(""));
                        d.text.push('\n');
                    }
                    if b["has_children"] == true {
                        queue.push(b["id"].as_str().context("ID bloc manquant")?.into())
                    }
                    if matches!(kind, "file" | "image" | "pdf") {
                        let url = value["file"]["url"]
                            .as_str()
                            .or_else(|| value["external"]["url"].as_str())
                            .context("URL pièce jointe absente")?;
                        let url = reqwest::Url::parse(url)?;
                        let host = url.host_str().unwrap_or("");
                        if url.scheme() != "https"
                            || !(host.ends_with(".amazonaws.com")
                                || host.ends_with(".notion.so")
                                || host.ends_with(".notionusercontent.com"))
                        {
                            bail!("Hôte de pièce jointe non autorisé ; import manuel requis")
                        }
                        let r = self
                            .client
                            .get(url.clone())
                            .send()
                            .await?
                            .error_for_status()?;
                        if r.content_length().unwrap_or(0) > 20 * 1024 * 1024 {
                            bail!("Pièce jointe trop grande")
                        }
                        let bytes = r.bytes().await?;
                        if bytes.len() > 20 * 1024 * 1024 {
                            bail!("Pièce jointe trop grande")
                        }
                        let name = if bytes.starts_with(b"%PDF") {
                            "source.pdf"
                        } else if bytes.starts_with(b"\x89PNG") {
                            "source.png"
                        } else if bytes.starts_with(&[255, 216]) {
                            "source.jpg"
                        } else {
                            bail!("Pièce jointe : format non reconnu")
                        };
                        let path = store.root.join("originals").join(hash(&bytes)).join(name);
                        std::fs::create_dir_all(path.parent().unwrap())?;
                        if !path.exists() {
                            std::fs::write(&path, &bytes)?
                        }
                        let mut media = crate::ingestion::import_with_refs(
                            store,
                            &path,
                            &source.subject,
                            &source.chapter,
                            source.kind,
                            source.level,
                            &source.reason,
                            Some(b["id"].as_str().context("ID pièce jointe absent")?),
                            &source.references,
                        )?;
                        media.title = format!("{} — pièce jointe", source.chapter);
                        attachments.push(media);
                    }
                    if kind == "unsupported" {
                        d.uncertainties
                            .push("Bloc Notion non pris en charge".into())
                    }
                }
            }
            d.content_hash = hash(d.text.as_bytes());
            let archive = store
                .root
                .join("originals")
                .join(&d.content_hash)
                .join(format!("{}.notion.json", d.id));
            std::fs::create_dir_all(archive.parent().unwrap())?;
            if !archive.exists() {
                std::fs::write(archive, serde_json::to_vec_pretty(&d)?)?;
            }
            // Empty container pages are metadata, not invented learning material.
            if !d.text.trim().is_empty() || d.deleted {
                docs.push(d)
            }
            docs.extend(attachments);
        }
        let current = docs.iter().map(|d| d.id.clone()).collect::<Vec<_>>();
        let mut previous = store
            .get::<Vec<String>>("inventory", "notion")?
            .unwrap_or_default();
        previous.extend(
            store
                .get::<Vec<String>>("inventory-pending", "notion")?
                .unwrap_or_default(),
        );
        previous.sort();
        previous.dedup();
        {
            for old in previous {
                if !current.contains(&old) {
                    if let Some(mut d) = store.get::<Document>("document", &old)? {
                        d.deleted = true;
                        docs.push(d);
                    }
                }
            }
        }
        // This inventory is only promoted after the entire pipeline succeeds.
        store.put("inventory-pending", "notion", &current, false)?;
        Ok(docs)
    }
    pub async fn publish_fiche(&self, store: &Store, config: &Config, f: &Fiche) -> Result<String> {
        if f.status != Status::Accepted {
            bail!("Fiche non validée")
        };
        store.validate_sources(&f.sources)?;
        let parent = config
            .notion_destinations
            .get(&format!("{}/{}", f.subject, f.chapter))
            .context("Destination chapitre existante requise")?;
        let mut identity = f.clone();
        identity.notion_url.clear();
        let digest = hash(serde_json::to_vec(&identity)?);
        let marker = format!("{} · {}", f.title, &digest[..12]);
        let key = format!("notion:{}:{digest}", f.id);
        // Reconcile a potentially successful previous request before ever retrying a create.
        for b in self.children(parent).await? {
            if b["type"] == "child_page" && b["child_page"]["title"] == marker {
                let url = format!(
                    "https://www.notion.so/{}",
                    b["id"].as_str().context("ID page absent")?.replace('-', "")
                );
                store.publication(&key, "success", &url)?;
                return Ok(url);
            }
        }
        if let Some((status, _)) = store.publication_status(&key)? {
            if status == "attempting" {
                bail!(
                    "Écriture Notion ambiguë : vérifier la destination avant une nouvelle tentative"
                )
            }
        }
        let mut blocks = crate::export::notion_blocks(f);
        for cite in &f.sources {
            if let Some(d) = store.get::<Document>("document", &cite.document_id)? {
                if d.original.starts_with("https://www.notion.so/") {
                    blocks.push(json!({"object":"block","type":"paragraph","paragraph":{"rich_text":[{"type":"text","text":{"content":format!("Source : {} (p. {})",d.title,cite.page),"link":{"url":d.original}}}]}}));
                }
            }
        }
        if blocks.len() > 100 {
            bail!("Fiche trop longue")
        }
        store.publication(&key, "attempting", parent)?;
        let page=self.request(reqwest::Method::POST,"pages",Some(json!({"parent":{"page_id":parent},"properties":{"title":{"type":"title","title":[{"text":{"content":marker}}]}},"children":blocks}))).await?;
        let url = page["url"].as_str().context("URL page absente")?.to_owned();
        store.publication(&key, "success", &url)?;
        Ok(url)
    }
}
pub struct Github {
    client: reqwest::Client,
    token: String,
    repo: String,
    base: String,
}
impl Github {
    pub fn from_env(repo: &str) -> Result<Self> {
        if !repo.starts_with("CinnamonVII/") || repo.split('/').count() != 2 {
            bail!("Dépôt dédié CinnamonVII requis")
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
            token: std::env::var("GITHUB_TOKEN")?,
            repo: repo.into(),
            base: "https://api.github.com".into(),
        })
    }
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let mut req = self
            .client
            .request(method, format!("{}/repos/{}{path}", self.base, self.repo))
            .header("User-Agent", "study-platform")
            .header("Accept", "application/vnd.github+json")
            .bearer_auth(&self.token);
        if let Some(v) = body {
            req = req.json(&v)
        }
        let r = req.send().await?;
        if !r.status().is_success() {
            bail!("GitHub HTTP {}", r.status())
        }
        Ok(r.json().await?)
    }
    pub async fn publish(&self, store: &Store) -> Result<String> {
        let meta = self.request(reqwest::Method::GET, "", None).await?;
        if meta["private"] != true {
            bail!("Publication refusée : le dépôt doit être privé")
        }
        let mut snapshot = store.snapshot()?;
        snapshot.fiches.retain(|f| f.status == Status::Accepted);
        snapshot.questions.retain(|q| q.status == Status::Accepted);
        for f in &snapshot.fiches {
            store.validate_sources(&f.sources)?;
        }
        for q in &snapshot.questions {
            store.validate_sources(&q.sources)?;
        }
        for d in &mut snapshot.documents {
            d.text.clear();
            if !d.original.starts_with("https://www.notion.so/") {
                d.original.clear()
            }
        }
        snapshot.progress.clear();
        snapshot.runs.clear();
        let data = serde_json::to_vec_pretty(&snapshot)?;
        let digest = hash(&data);
        let branch = format!("study/generated-{}", &digest[..16]);
        let key = format!("github:{digest}");
        if let Some((status, url)) = store.publication_status(&key)? {
            if status == "success" {
                return Ok(url);
            }
        }
        let default = meta["default_branch"]
            .as_str()
            .context("Branche principale absente")?;
        let head = self
            .request(
                reqwest::Method::GET,
                &format!("/git/ref/heads/{default}"),
                None,
            )
            .await?;
        let sha = head["object"]["sha"].as_str().context("SHA absent")?;
        let commit = self
            .request(reqwest::Method::GET, &format!("/git/commits/{sha}"), None)
            .await?;
        let mut entries = vec![
            json!({"path":"content/metadata/snapshot.json","mode":"100644","type":"blob","content":String::from_utf8(data)?}),
        ];
        for q in &snapshot.questions {
            entries.push(json!({"path":format!("content/questions/{}.json",q.id),"mode":"100644","type":"blob","content":serde_json::to_string_pretty(q)?}));
        }
        for f in &snapshot.fiches {
            let out = store.root.join("exports").join(&f.id);
            let source = crate::export::typst(f);
            if std::fs::read_to_string(out.join("fiche.typ"))
                .ok()
                .as_deref()
                != Some(&source)
                || !out.join("fiche.pdf").exists()
            {
                crate::export::export(f, &store.root.join("exports"), true)?;
            }
            entries.push(json!({"path":format!("content/fiches/{}/fiche.json",f.id),"mode":"100644","type":"blob","content":serde_json::to_string_pretty(f)?}));
            for ext in ["md", "tex", "typ"] {
                entries.push(json!({"path":format!("content/fiches/{}/fiche.{ext}",f.id),"mode":"100644","type":"blob","content":std::fs::read_to_string(out.join(format!("fiche.{ext}")))?}));
            }
            let blob=self.request(reqwest::Method::POST,"/git/blobs",Some(json!({"encoding":"base64","content":base64::engine::general_purpose::STANDARD.encode(std::fs::read(out.join("fiche.pdf"))?)}))).await?;
            entries.push(json!({"path":format!("content/fiches/{}/fiche.pdf",f.id),"mode":"100644","type":"blob","sha":blob["sha"]}));
        }
        let tree = self
            .request(
                reqwest::Method::POST,
                "/git/trees",
                Some(json!({"base_tree":commit["tree"]["sha"],"tree":entries})),
            )
            .await?;
        let commit=self.request(reqwest::Method::POST,"/git/commits",Some(json!({"message":"study: proposer les révisions validées","tree":tree["sha"],"parents":[sha]}))).await?;
        store.publication(&key, "attempting", &branch)?;
        let created = self
            .request(
                reqwest::Method::POST,
                "/git/refs",
                Some(json!({"ref":format!("refs/heads/{branch}"),"sha":commit["sha"]})),
            )
            .await;
        if created.is_err() {
            let existing = self
                .request(
                    reqwest::Method::GET,
                    &format!("/contents/content/metadata/snapshot.json?ref={branch}"),
                    None,
                )
                .await?;
            let encoded = existing["content"].as_str().unwrap_or("").replace('\n', "");
            let bytes = base64::engine::general_purpose::STANDARD.decode(encoded)?;
            if hash(bytes) != digest {
                bail!("Conflit GitHub ; aucune branche écrasée")
            }
        }
        let prs = self
            .request(
                reqwest::Method::GET,
                &format!("/pulls?head=CinnamonVII:{branch}&state=open"),
                None,
            )
            .await?;
        let url = if let Some(url) = prs[0]["html_url"].as_str() {
            url.into()
        } else {
            let pr=self.request(reqwest::Method::POST,"/pulls",Some(json!({"title":"Révisions : contenus validés à intégrer","head":branch,"base":default,"body":"Instantané privé validé par le moteur. Les sources originales sont conservées hors Git. Vérifier les corrections manuelles avant fusion.","draft":true}))).await?;
            pr["html_url"]
                .as_str()
                .context("URL PR absente")?
                .to_owned()
        };
        store.publication(&key, "success", &url)?;
        Ok(url)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };
    fn store() -> (tempfile::TempDir, Store) {
        let t = tempfile::tempdir().unwrap();
        let s = Store::open(t.path()).unwrap();
        (t, s)
    }
    #[tokio::test]
    async fn github_public_repository_is_never_written() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/CinnamonVII/study-platform-data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"private":false})))
            .expect(1)
            .mount(&mock)
            .await;
        let (_t, s) = store();
        let g = Github {
            client: reqwest::Client::new(),
            token: "fake".into(),
            repo: "CinnamonVII/study-platform-data".into(),
            base: mock.uri(),
        };
        assert!(
            g.publish(&s)
                .await
                .unwrap_err()
                .to_string()
                .contains("privé")
        );
        assert!(
            s.db()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM publications", [], |r| r
                    .get::<_, i64>(0))
                .unwrap()
                == 0
        );
    }
    #[tokio::test]
    async fn github_write_failure_does_not_mark_publication_successful() {
        let mock = MockServer::start().await;
        let prefix = "/repos/CinnamonVII/study-platform-data";
        for (p, body) in [
            (
                prefix.to_owned(),
                json!({"private":true,"default_branch":"main"}),
            ),
            (
                format!("{prefix}/git/ref/heads/main"),
                json!({"object":{"sha":"abc"}}),
            ),
            (
                format!("{prefix}/git/commits/abc"),
                json!({"tree":{"sha":"tree"}}),
            ),
        ] {
            Mock::given(method("GET"))
                .and(path(p))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&mock)
                .await;
        }
        Mock::given(method("POST"))
            .and(path(format!("{prefix}/git/trees")))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock)
            .await;
        let (_t, s) = store();
        let g = Github {
            client: reqwest::Client::new(),
            token: "fake".into(),
            repo: "CinnamonVII/study-platform-data".into(),
            base: mock.uri(),
        };
        assert!(g.publish(&s).await.is_err());
        assert_eq!(
            s.db()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM publications WHERE status='success'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn ambiguous_notion_write_is_not_blindly_retried() {
        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/blocks/parent/children"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"results":[],"has_more":false})),
            )
            .expect(2)
            .mount(&mock)
            .await;
        Mock::given(method("POST"))
            .and(path("/pages"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock)
            .await;
        let (_t, s) = store();
        let demo: Snapshot =
            serde_json::from_str(include_str!("../../../fixtures/demo/snapshot.json")).unwrap();
        for d in &demo.documents {
            s.put("document", &d.id, d, true).unwrap();
        }
        let f = &demo.fiches[0];
        let mut cfg = Config::default();
        cfg.notion_destinations
            .insert(format!("{}/{}", f.subject, f.chapter), "parent".into());
        let n = Notion {
            client: reqwest::Client::new(),
            token: "fake".into(),
            base: mock.uri(),
        };
        assert!(n.publish_fiche(&s, &cfg, f).await.is_err());
        assert!(
            n.publish_fiche(&s, &cfg, f)
                .await
                .unwrap_err()
                .to_string()
                .contains("ambiguë")
        );
        assert_eq!(
            s.db()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM publications WHERE status='success'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
}
#[cfg(test)]
mod inventory_tests {
    use super::*;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };
    #[tokio::test]
    async fn removed_notion_attachment_is_withdrawn_after_complete_scan() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pages/page"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"last_edited_time":"2026-11-03T07:00:00Z","archived":false}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/blocks/page/children"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"results":[],"has_more":false})),
            )
            .mount(&server)
            .await;
        let t = tempfile::tempdir().unwrap();
        let s = Store::open(t.path()).unwrap();
        let mut docs: Vec<Document> =
            serde_json::from_str(include_str!("../../../fixtures/demo/documents.json")).unwrap();
        let d = docs.remove(0);
        s.put("document", &d.id, &d, true).unwrap();
        s.put("inventory", "notion", &vec![d.id.clone()], false)
            .unwrap();
        let n = Notion {
            client: reqwest::Client::new(),
            token: "fake".into(),
            base: server.uri(),
        };
        let cfg = Config {
            notion_sources: vec![NotionSource {
                page_id: "page".into(),
                subject: "SES".into(),
                chapter: "EC3".into(),
                kind: DocType::Cours,
                level: Some(1),
                reason: "Personnel confirmé".into(),
                references: vec![],
            }],
            ..Default::default()
        };
        let docs = n.scan(&s, &cfg).await.unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].deleted);
        assert_eq!(docs[0].id, d.id);
    }
    #[tokio::test]
    async fn removed_attachment_from_failed_first_scan_is_withdrawn() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pages/page"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    json!({"last_edited_time":"2026-11-03T07:00:00Z","archived":false}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/blocks/page/children"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"results":[],"has_more":false})),
            )
            .mount(&server)
            .await;
        let t = tempfile::tempdir().unwrap();
        let s = Store::open(t.path()).unwrap();
        let mut docs: Vec<Document> =
            serde_json::from_str(include_str!("../../../fixtures/demo/documents.json")).unwrap();
        let d = docs.remove(0);
        s.put("document", &d.id, &d, true).unwrap();
        s.put("inventory-pending", "notion", &vec![d.id.clone()], false)
            .unwrap();
        let n = Notion {
            client: reqwest::Client::new(),
            token: "fake".into(),
            base: server.uri(),
        };
        let cfg = Config {
            notion_sources: vec![NotionSource {
                page_id: "page".into(),
                subject: "SES".into(),
                chapter: "EC3".into(),
                kind: DocType::Cours,
                level: Some(1),
                reason: "Personnel confirmé".into(),
                references: vec![],
            }],
            ..Default::default()
        };
        let docs = n.scan(&s, &cfg).await.unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].deleted);
        assert_eq!(docs[0].id, d.id);
    }
    #[tokio::test]
    async fn notion_text_is_archived_immutably() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pages/page"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"archived":false})))
            .mount(&server)
            .await;
        Mock::given(method("GET")).and(path("/blocks/page/children")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":[{"type":"paragraph","paragraph":{"rich_text":[{"plain_text":"Texte original fictif"}]}}],"has_more":false}))).mount(&server).await;
        let t = tempfile::tempdir().unwrap();
        let s = Store::open(t.path()).unwrap();
        let n = Notion {
            client: reqwest::Client::new(),
            token: "fake".into(),
            base: server.uri(),
        };
        let cfg = Config {
            notion_sources: vec![NotionSource {
                page_id: "page".into(),
                subject: "SES".into(),
                chapter: "EC3".into(),
                kind: DocType::Cours,
                level: Some(1),
                reason: "Personnel confirmé".into(),
                references: vec![],
            }],
            ..Default::default()
        };
        let docs = n.scan(&s, &cfg).await.unwrap();
        let archive = s
            .root
            .join("originals")
            .join(&docs[0].content_hash)
            .join("page.notion.json");
        let archived: Document = serde_json::from_slice(&std::fs::read(archive).unwrap()).unwrap();
        assert_eq!(archived.text, "Texte original fictif\n");
    }
}

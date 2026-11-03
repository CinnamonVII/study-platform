use anyhow::{Result, bail};
use fs2::FileExt;
use rusqlite::{Connection, params};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};
use study_domain::*;
#[derive(Clone)]
pub struct Store {
    pub root: PathBuf,
}
pub struct Guard(File);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}
impl Store {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let s = Self {
            root: root.as_ref().to_owned(),
        };
        fs::create_dir_all(&s.root)?;
        s.db()?.execute_batch("PRAGMA journal_mode=WAL;
        CREATE TABLE IF NOT EXISTS objects(kind TEXT,id TEXT,payload TEXT,manual INTEGER DEFAULT 0,PRIMARY KEY(kind,id));
        CREATE TABLE IF NOT EXISTS checkpoints(id TEXT PRIMARY KEY,hash TEXT);
        CREATE TABLE IF NOT EXISTS runs(id INTEGER PRIMARY KEY,started TEXT,status TEXT,detail TEXT);
        CREATE TABLE IF NOT EXISTS publications(id TEXT PRIMARY KEY,status TEXT,detail TEXT);")?;
        Ok(s)
    }
    pub fn db(&self) -> Result<Connection> {
        let c = Connection::open(self.root.join("study.sqlite"))?;
        c.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(c)
    }
    pub fn lock(&self) -> Result<Guard> {
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("sync.lock"))?;
        f.try_lock_exclusive()
            .map_err(|_| anyhow::anyhow!("Une synchronisation est déjà active"))?;
        Ok(Guard(f))
    }
    pub fn list<T: DeserializeOwned>(&self, kind: &str) -> Result<Vec<T>> {
        let c = self.db()?;
        let mut st = c.prepare("SELECT payload FROM objects WHERE kind=? ORDER BY id")?;
        st.query_map([kind], |r| r.get::<_, String>(0))?
            .map(|r| Ok(serde_json::from_str(&r?)?))
            .collect()
    }
    pub fn get<T: DeserializeOwned>(&self, kind: &str, id: &str) -> Result<Option<T>> {
        use rusqlite::OptionalExtension;
        let v: Option<String> = self
            .db()?
            .query_row(
                "SELECT payload FROM objects WHERE kind=? AND id=?",
                params![kind, id],
                |r| r.get(0),
            )
            .optional()?;
        v.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
    }
    pub fn put<T: Serialize>(&self, kind: &str, id: &str, obj: &T, manual: bool) -> Result<()> {
        Self::put_in(&self.db()?, kind, id, obj, manual, false)
    }
    pub fn put_in<T: Serialize>(
        c: &Connection,
        kind: &str,
        id: &str,
        obj: &T,
        manual: bool,
        force: bool,
    ) -> Result<()> {
        let protected: bool = c.query_row(
            "SELECT COALESCE(MAX(manual),0) FROM objects WHERE kind=? AND id=?",
            params![kind, id],
            |r| r.get(0),
        )?;
        let kind = if protected && !manual && !force {
            format!("candidate-{kind}")
        } else {
            kind.to_owned()
        };
        c.execute(
            "INSERT OR REPLACE INTO objects VALUES(?,?,?,?)",
            params![
                kind,
                id,
                serde_json::to_string(obj)?,
                if protected || manual { 1 } else { 0 }
            ],
        )?;
        Ok(())
    }
    pub fn is_manual(&self, kind: &str, id: &str) -> Result<bool> {
        Ok(self.db()?.query_row(
            "SELECT COALESCE(MAX(manual),0) FROM objects WHERE kind=? AND id=?",
            params![kind, id],
            |r| r.get(0),
        )?)
    }
    pub fn checkpoint(&self, id: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .db()?
            .query_row("SELECT hash FROM checkpoints WHERE id=?", [id], |r| {
                r.get(0)
            })
            .optional()?)
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let c = self.db()?;
        let mut st =
            c.prepare("SELECT id,started,status,detail FROM runs ORDER BY id DESC LIMIT 30")?;
        let runs = st
            .query_map([], |r| {
                Ok(Run {
                    id: r.get(0)?,
                    started: r.get(1)?,
                    status: r.get(2)?,
                    detail: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Snapshot {
            schema_version: "1.0".into(),
            documents: self.list("document")?,
            fiches: self.list("fiche")?,
            questions: self.list("question")?,
            runs,
            progress: self.list("progress")?,
        })
    }
    pub fn start_run(&self) -> Result<i64> {
        let c = self.db()?;
        c.execute(
            "UPDATE runs SET status='interrupted' WHERE status='running'",
            [],
        )?;
        c.execute(
            "INSERT INTO runs(started,status,detail) VALUES(?,'running','')",
            [now()],
        )?;
        Ok(c.last_insert_rowid())
    }
    pub fn finish_run(&self, id: i64, status: &str, detail: &str) -> Result<()> {
        self.db()?.execute(
            "UPDATE runs SET status=?,detail=? WHERE id=?",
            params![status, detail, id],
        )?;
        Ok(())
    }
    pub fn validate_sources(&self, sources: &[Citation]) -> Result<()> {
        if sources.is_empty() {
            bail!("Références requises")
        }
        for s in sources {
            let d = self
                .get::<Document>("document", &s.document_id)?
                .ok_or_else(|| anyhow::anyhow!("Source absente"))?;
            if s.page == 0 || s.quote.trim().is_empty() || !d.text.contains(&s.quote) {
                bail!("Citation absente de la version actuelle de la source")
            }
            if d.deleted || d.status != Status::Accepted || !d.uncertainties.is_empty() {
                bail!("Source à vérifier : {}", d.title)
            }
        }
        Ok(())
    }
    pub fn accept(&self, kind: &str, id: &str, status: Status) -> Result<()> {
        let _guard = self.lock()?;
        match kind {
            "document" => {
                let mut d = self
                    .get::<Document>(kind, id)?
                    .ok_or_else(|| anyhow::anyhow!("Document absent"))?;
                d.status = status;
                d.validate().map_err(anyhow::Error::msg)?;
                self.put(kind, id, &d, true)?;
                if status != Status::Accepted {
                    for mut q in self.list::<Question>("question")? {
                        if q.sources.iter().any(|c| c.document_id == id) {
                            q.status = Status::Stale;
                            Self::put_in(&self.db()?, "question", &q.id, &q, false, true)?;
                        }
                    }
                    for mut f in self.list::<Fiche>("fiche")? {
                        if f.sources.iter().any(|c| c.document_id == id) {
                            f.status = Status::Stale;
                            Self::put_in(&self.db()?, "fiche", &f.id, &f, false, true)?;
                        }
                    }
                }
            }
            "fiche" => {
                let mut f = self
                    .get::<Fiche>(kind, id)?
                    .ok_or_else(|| anyhow::anyhow!("Fiche absente"))?;
                if status == Status::Accepted {
                    self.validate_sources(&f.sources)?;
                }
                f.validate().map_err(anyhow::Error::msg)?;
                f.status = status;
                self.put(kind, id, &f, true)?;
            }
            "question" => {
                let mut q = self
                    .get::<Question>(kind, id)?
                    .ok_or_else(|| anyhow::anyhow!("Question absente"))?;
                if status == Status::Accepted {
                    self.validate_sources(&q.sources)?;
                }
                q.validate().map_err(anyhow::Error::msg)?;
                q.status = status;
                self.put(kind, id, &q, true)?;
            }
            _ => bail!("Type de validation inconnu"),
        }
        Ok(())
    }
    pub fn publication(&self, key: &str, status: &str, detail: &str) -> Result<()> {
        self.db()?.execute(
            "INSERT OR REPLACE INTO publications VALUES(?,?,?)",
            params![key, status, detail],
        )?;
        Ok(())
    }
    pub fn publication_status(&self, key: &str) -> Result<Option<(String, String)>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .db()?
            .query_row(
                "SELECT status,detail FROM publications WHERE id=?",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }
}
impl Store {
    pub fn apply_candidate(&self, kind: &str, id: &str, apply: bool) -> Result<()> {
        if !["document", "fiche", "question"].contains(&kind) {
            bail!("Type de proposition inconnu")
        }
        let _guard = self.lock()?;
        let candidate_kind = format!("candidate-{kind}");
        let mut value = self
            .get::<serde_json::Value>(&candidate_kind, id)?
            .ok_or_else(|| anyhow::anyhow!("Proposition absente"))?;
        let mut c = self.db()?;
        let tx = c.transaction()?;
        if apply {
            if let Some(old) = self.get::<serde_json::Value>(kind, id)? {
                Self::put_in(
                    &tx,
                    &format!("history-{kind}"),
                    &format!("{id}-{}", now()),
                    &old,
                    true,
                    true,
                )?;
            }
            value["status"] = serde_json::json!("pending");
            if kind == "document" {
                serde_json::from_value::<Document>(value.clone())?
                    .validate()
                    .map_err(anyhow::Error::msg)?;
            }
            Self::put_in(&tx, kind, id, &value, true, true)?;
        }
        tx.execute(
            "DELETE FROM objects WHERE kind=? AND id=?",
            params![candidate_kind, id],
        )?;
        tx.commit()?;
        Ok(())
    }
}

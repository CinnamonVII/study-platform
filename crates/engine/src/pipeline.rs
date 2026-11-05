use crate::{Store, generation::Generator};
use anyhow::{Result, bail};
use rusqlite::params;
use study_domain::*;
#[derive(serde::Serialize)]
pub struct Report {
    pub changed: usize,
    pub run_id: i64,
}
pub async fn sync(store: &Store, g: &dyn Generator, documents: Vec<Document>) -> Result<Report> {
    let _guard = store.lock()?;
    sync_locked(store, g, documents).await
}
async fn sync_locked(store: &Store, g: &dyn Generator, documents: Vec<Document>) -> Result<Report> {
    let run = store.start_run()?;
    let result = process(store, g, documents).await;
    match result {
        Ok(n) => {
            if let Some(inventory) = store.get::<Vec<String>>("inventory-pending", "notion")? {
                store.put("inventory", "notion", &inventory, false)?;
            }
            store.finish_run(run, "success", &format!("{n} document(s) traité(s)"))?;
            Ok(Report {
                changed: n,
                run_id: run,
            })
        }
        Err(e) => {
            store.finish_run(
                run,
                "failed",
                "Traitement interrompu ; relance possible, consulter la console locale",
            )?;
            Err(e)
        }
    }
}
async fn process(store: &Store, g: &dyn Generator, docs: Vec<Document>) -> Result<usize> {
    let mut count = 0;
    let mut dirty = Vec::new();
    for incoming in docs {
        incoming.validate().map_err(anyhow::Error::msg)?;
        let mut d = incoming;
        // A corrected transcription stays attached to the same immutable source revision.
        if let Some(old) = store.get::<Document>("document", &d.id)? {
            if store.is_manual("document", &d.id)? {
                let metadata_changed = d.level != old.level
                    || d.kind != old.kind
                    || d.subject != old.subject
                    || d.chapter != old.chapter
                    || d.provenance_reason != old.provenance_reason;
                if metadata_changed {
                    store.put("candidate-document", &d.id, &d, false)?;
                }
                d.level = old.level;
                d.kind = old.kind;
                d.subject = old.subject.clone();
                d.chapter = old.chapter.clone();
                d.provenance_reason = old.provenance_reason.clone();
                d.title = old.title.clone();
                d.sources = old.sources.clone();
            }
            if old.content_hash == d.content_hash {
                d.transcription = old.transcription;
                d.text = old.text;
                d.uncertainties = old.uncertainties;
                d.status = old.status;
            }
        }
        let key = d.dependency_hash();
        if store.checkpoint(&d.id)?.as_deref() == Some(&key) {
            continue;
        }
        dirty.push(d);
    }
    // Make every known source change visible before any fallible model request.
    {
        let mut c = store.db()?;
        let tx = c.transaction()?;
        for d in &dirty {
            Store::put_in(&tx, "document", &d.id, d, false, true)?;
        }
        for mut q in store.list::<Question>("question")? {
            if q.sources
                .iter()
                .any(|s| dirty.iter().any(|d| d.id == s.document_id))
            {
                q.status = Status::Stale;
                Store::put_in(&tx, "question", &q.id, &q, false, true)?;
            }
        }
        for mut f in store.list::<Fiche>("fiche")? {
            if f.sources
                .iter()
                .any(|s| dirty.iter().any(|d| d.id == s.document_id))
            {
                f.status = Status::Stale;
                Store::put_in(&tx, "fiche", &f.id, &f, false, true)?;
            }
        }
        tx.commit()?;
    }
    for mut d in dirty {
        let eligible = d.level == Some(1)
            || matches!(
                d.kind,
                DocType::Devoir
                    | DocType::Exercice
                    | DocType::Correction
                    | DocType::Methode
                    | DocType::Annale
            )
            || (d.level == Some(2) && d.status == Status::Accepted);
        let mut output = None;
        if eligible && !d.deleted && d.status != Status::Rejected {
            let media = [".png", ".jpg", ".jpeg", ".pdf"]
                .iter()
                .any(|e| d.original.to_lowercase().ends_with(e));
            if media && d.status != Status::Accepted {
                let t = g.transcribe(&d).await?;
                for cite in &t.citations {
                    if cite.document_id != d.id
                        || cite.page == 0
                        || cite.quote.is_empty()
                        || !t.text.contains(&cite.quote)
                    {
                        bail!("Référence de transcription invalide")
                    }
                }
                d.transcription = Some(t.clone());
                d.text = t.text;
                d.uncertainties = t.uncertainties;
            }
            if d.text.trim().is_empty() {
                bail!("Source vide : {}", d.title)
            }
            let mut context = d.clone();
            if d.level == Some(2) {
                store.validate_sources(&d.sources)?;
                for cite in &d.sources {
                    let root = store
                        .get::<Document>("document", &cite.document_id)?
                        .ok_or_else(|| anyhow::anyhow!("Original manquant"))?;
                    if root.level != Some(1) {
                        bail!("Une fiche dérivée doit être vérifiée dans un original de niveau 1")
                    }
                    context
                        .text
                        .push_str(&format!("\nOriginal {} :\n{}", root.id, root.text));
                }
            }
            let mut out = g.generate(&context).await?;
            validate_generated(&context, &out)?;
            for cites in
                std::iter::once(&out.fiche.sources).chain(out.questions.iter().map(|q| &q.sources))
            {
                for cite in cites {
                    if cite.document_id != d.id {
                        let original = store
                            .get::<Document>("document", &cite.document_id)?
                            .ok_or_else(|| anyhow::anyhow!("Original cité absent"))?;
                        if !original.text.contains(&cite.quote) {
                            bail!("Citation absente de l'original")
                        }
                    }
                }
                if d.level == Some(2)
                    && !cites
                        .iter()
                        .any(|c| d.sources.iter().any(|s| s.document_id == c.document_id))
                {
                    bail!("La sortie doit citer les originaux")
                }
            }
            out.fiche.id = id(&[&d.id, "fiche"]);
            out.fiche.status = Status::Pending;
            let old: Vec<Question> = store.list("question")?;
            let mut dedup = Vec::<Question>::new();
            for mut q in out.questions {
                if dedup.iter().any(|o| similar(&o.prompt, &q.prompt)) {
                    continue;
                }
                let matched = old.iter().find(|o| {
                    o.subject == q.subject
                        && o.chapter == q.chapter
                        && similar(&o.prompt, &q.prompt)
                });
                q.id = matched.map(|q| q.id.clone()).unwrap_or_else(|| {
                    id(&[&d.id, &format!("{:?}", q.kind), &q.prompt.to_lowercase()])
                });
                if let Some(m) = matched {
                    q.created_at = m.created_at.clone();
                    for c in &m.sources {
                        let valid = if c.document_id == d.id {
                            d.text.contains(&c.quote)
                        } else {
                            store
                                .get::<Document>("document", &c.document_id)?
                                .is_some_and(|r| !r.deleted && r.text.contains(&c.quote))
                        };
                        if valid && !q.sources.contains(c) {
                            q.sources.push(c.clone());
                        }
                    }
                }
                q.updated_at = now();
                q.status = Status::Pending;
                q.fiche_ids = vec![out.fiche.id.clone()];
                dedup.push(q);
            }
            out.questions = dedup;
            d.uncertainties.extend(out.uncertainties.clone());
            d.uncertainties.sort();
            d.uncertainties.dedup();
            output = Some(out);
        }
        let mut c = store.db()?;
        let tx = c.transaction()?;
        // Changed or withdrawn sources invalidate ALL dependent accepted material, even manual edits.
        for mut q in store.list::<Question>("question")? {
            if q.sources.iter().any(|s| s.document_id == d.id) {
                q.status = Status::Stale;
                Store::put_in(&tx, "question", &q.id, &q, false, true)?;
            }
        }
        for mut f in store.list::<Fiche>("fiche")? {
            if f.sources.iter().any(|s| s.document_id == d.id) {
                f.status = Status::Stale;
                Store::put_in(&tx, "fiche", &f.id, &f, false, true)?;
            }
        }
        Store::put_in(&tx, "document", &d.id, &d, false, true)?;
        if let Some(out) = output {
            if d.uncertainties.is_empty() {
                Store::put_in(&tx, "fiche", &out.fiche.id, &out.fiche, false, false)?;
                for q in out.questions {
                    Store::put_in(&tx, "question", &q.id, &q, false, false)?;
                }
            } else {
                Store::put_in(
                    &tx,
                    "candidate-fiche",
                    &out.fiche.id,
                    &out.fiche,
                    false,
                    true,
                )?;
            }
        }
        // Persist the post-transcription fingerprint: next identical run performs no AI call.
        tx.execute(
            "INSERT OR REPLACE INTO checkpoints VALUES(?,?)",
            params![d.id, d.dependency_hash()],
        )?;
        tx.commit()?;
        // Save each completed document before starting another paid model request.
        // A terminated disposable runner can then resume without losing earlier work.
        store.checkpoint_remote()?;
        count += 1;
    }
    Ok(count)
}
pub fn validate_generated(d: &Document, out: &Generated) -> Result<()> {
    out.fiche.validate().map_err(anyhow::Error::msg)?;
    for q in &out.questions {
        q.validate().map_err(anyhow::Error::msg)?;
    }
    if out.fiche.level != 2 || out.fiche.sources.is_empty() || out.questions.is_empty() {
        bail!("Sortie incomplète ou provenance incorrecte")
    }
    for sources in
        std::iter::once(&out.fiche.sources).chain(out.questions.iter().map(|q| &q.sources))
    {
        if sources.is_empty() {
            bail!("Citation absente")
        }
        for c in sources {
            if c.document_id != d.id && !d.sources.iter().any(|s| s.document_id == c.document_id) {
                bail!("Source inventée")
            }
            if c.page == 0
                || c.quote.trim().is_empty()
                || (c.document_id == d.id && !d.text.contains(&c.quote))
            {
                bail!("Citation introuvable")
            }
        }
    }
    if out.fiche.sections.is_empty()
        || out.fiche.essentials.is_empty()
        || out.fiche.recall.is_empty()
        || out.fiche.subject != d.subject
        || out.fiche.chapter != d.chapter
    {
        bail!("Structure ou classement de fiche invalide")
    }
    for q in &out.questions {
        if q.subject != d.subject
            || q.chapter != d.chapter
            || q.explanation.trim().is_empty()
            || ![
                "memorisation",
                "comprehension",
                "application",
                "raisonnement",
            ]
            .contains(&q.cognitive.as_str())
        {
            bail!("Métadonnées pédagogiques de question invalides")
        }
        if matches!(q.kind, QuestionType::Open | QuestionType::Method) && q.rubric.is_empty() {
            bail!("Grille d’évaluation requise")
        }
        if q.prompt.trim().is_empty()
            || q.answer.trim().is_empty()
            || !(1..=5).contains(&q.difficulty)
        {
            bail!("Question invalide")
        }
        if q.kind == QuestionType::Mcq && (q.choices.len() < 2 || !q.choices.contains(&q.answer)) {
            bail!("QCM invalide")
        }
    }
    Ok(())
}

pub async fn sync_notion(
    store: &Store,
    g: &dyn Generator,
    notion: &crate::connectors::Notion,
    config: &crate::connectors::Config,
) -> Result<Report> {
    let _guard = store.lock()?;
    let docs = match notion.scan(store, config).await {
        Ok(d) => d,
        Err(e) => {
            let r = store.start_run()?;
            store.finish_run(
                r,
                "failed",
                "Lecture Notion incomplète : aucun passage réussi enregistré",
            )?;
            return Err(e);
        }
    };
    sync_locked(store, g, docs).await
}

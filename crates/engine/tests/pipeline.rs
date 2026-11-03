use anyhow::Result;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use study_domain::*;
use study_engine::{
    Store,
    generation::{FixtureGenerator, Generator},
    ingestion::import,
    pipeline::sync,
};
fn documents() -> Vec<Document> {
    serde_json::from_str(include_str!("../../../fixtures/demo/documents.json")).unwrap()
}
fn setup() -> (tempfile::TempDir, Store) {
    let t = tempfile::tempdir().unwrap();
    let s = Store::open(t.path()).unwrap();
    (t, s)
}
struct Mock {
    calls: AtomicUsize,
    fail_at: usize,
    uncertain: bool,
    duplicate: bool,
}
impl Mock {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            fail_at: usize::MAX,
            uncertain: false,
            duplicate: false,
        }
    }
}
#[async_trait]
impl Generator for Mock {
    async fn transcribe(&self, d: &Document) -> Result<Transcription> {
        Ok(Transcription {
            text: if d.text.is_empty() {
                "u(n+1) = u(n) + r".into()
            } else {
                d.text.clone()
            },
            uncertainties: if self.uncertain {
                vec!["Signe + ou × illisible à la page 1".into()]
            } else {
                vec![]
            },
            citations: vec![],
        })
    }
    async fn generate(&self, d: &Document) -> Result<Generated> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == self.fail_at {
            anyhow::bail!("Interruption simulée")
        };
        let mut out = FixtureGenerator.generate(d).await?;
        if self.duplicate {
            out.questions.push(out.questions[0].clone())
        }
        Ok(out)
    }
}
#[tokio::test]
async fn handwritten_original_is_preserved_and_generates_pending_content() {
    let (t, s) = setup();
    let p = t.path().join("notes.png");
    std::fs::write(&p, include_bytes!("../../ui/icon-192.png")).unwrap();
    let d = import(
        &s,
        &p,
        "Mathématiques",
        "Suites",
        DocType::Cours,
        Some(1),
        "Photo de mon cahier",
        None,
    )
    .unwrap();
    let original = d.original.clone();
    let raw = std::fs::read(&p).unwrap();
    sync(&s, &Mock::new(), vec![d]).await.unwrap();
    assert_eq!(raw, std::fs::read(original).unwrap());
    assert_eq!(s.list::<Fiche>("fiche").unwrap()[0].level, 2);
    assert_eq!(
        s.list::<Question>("question").unwrap()[0].status,
        Status::Pending
    );
}
#[tokio::test]
async fn modified_source_updates_same_fiche_id() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let first = s.list::<Fiche>("fiche").unwrap()[0].id.clone();
    d.text.push_str("\nUne suite constante a pour raison 0.");
    d.content_hash = hash(&d.text);
    sync(&s, &Mock::new(), vec![d]).await.unwrap();
    assert_eq!(s.list::<Fiche>("fiche").unwrap().len(), 1);
    assert_eq!(s.list::<Fiche>("fiche").unwrap()[0].id, first);
}
#[tokio::test]
async fn homework_without_original_classification_is_processed() {
    let (_t, s) = setup();
    let mut d = documents().remove(1);
    d.level = None;
    d.provenance_reason.clear();
    sync(&s, &Mock::new(), vec![d]).await.unwrap();
    assert_eq!(s.list::<Question>("question").unwrap().len(), 1);
}
#[tokio::test]
async fn fiche_and_questions_retain_source_references() {
    let (_t, s) = setup();
    let d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let q = s.list::<Question>("question").unwrap().remove(0);
    let f = s.list::<Fiche>("fiche").unwrap().remove(0);
    assert_eq!(q.fiche_ids, vec![f.id]);
    assert_eq!(q.sources[0].document_id, d.id);
    assert!(d.text.contains(&q.sources[0].quote));
}
#[tokio::test]
async fn accepted_derived_fiche_is_checked_against_original() {
    let (_t, s) = setup();
    let mut original = documents().remove(0);
    original.status = Status::Accepted;
    s.put("document", &original.id, &original, true).unwrap();
    let mut derived = original.clone();
    derived.id = "derived".into();
    derived.kind = DocType::Fiche;
    derived.level = Some(2);
    derived.provenance_reason = "generated".into();
    derived.sources = vec![Citation {
        document_id: original.id.clone(),
        page: 1,
        quote: original.text.lines().next().unwrap().into(),
        region: None,
    }];
    sync(&s, &Mock::new(), vec![derived]).await.unwrap();
    assert_eq!(
        s.list::<Question>("question").unwrap()[0].sources[0].document_id,
        original.id
    );
}
#[tokio::test]
async fn semantic_duplicates_are_collapsed() {
    let (_t, s) = setup();
    let mut g = Mock::new();
    g.duplicate = true;
    sync(&s, &g, vec![documents().remove(0)]).await.unwrap();
    assert_eq!(s.list::<Question>("question").unwrap().len(), 1);
    assert!(similar(
        "Définir le prix d'équilibre.",
        "Définir le prix d’équilibre !"
    ));
}
#[tokio::test]
async fn unchanged_sync_makes_zero_generator_calls() {
    let (_t, s) = setup();
    let g = Mock::new();
    let d = documents().remove(0);
    assert_eq!(sync(&s, &g, vec![d.clone()]).await.unwrap().changed, 1);
    assert_eq!(sync(&s, &g, vec![d]).await.unwrap().changed, 0);
    assert_eq!(g.calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn interrupted_batch_resumes_only_unfinished_document() {
    let (_t, s) = setup();
    let mut g = Mock::new();
    g.fail_at = 1;
    let docs = documents()[..2].to_vec();
    assert!(sync(&s, &g, docs.clone()).await.is_err());
    assert_eq!(s.list::<Fiche>("fiche").unwrap().len(), 1);
    let next = Mock::new();
    assert_eq!(sync(&s, &next, docs).await.unwrap().changed, 1);
    assert_eq!(next.calls.load(Ordering::SeqCst), 1);
    assert!(
        s.snapshot()
            .unwrap()
            .runs
            .iter()
            .any(|r| r.status == "failed")
    );
}
#[tokio::test]
async fn ambiguous_handwritten_formula_never_enters_training() {
    let (t, s) = setup();
    let mut d = documents().remove(0);
    let p = t.path().join("formula.png");
    std::fs::write(&p, include_bytes!("../../ui/icon-192.png")).unwrap();
    d.original = p.to_string_lossy().into();
    let mut g = Mock::new();
    g.uncertain = true;
    sync(&s, &g, vec![d]).await.unwrap();
    assert!(training_questions(&s.snapshot().unwrap()).is_empty());
    assert_eq!(s.list::<Fiche>("candidate-fiche").unwrap().len(), 1);
    assert!(
        s.accept("document", "demo-maths", Status::Accepted)
            .is_err()
    );
}
#[tokio::test]
async fn manual_correction_is_kept_as_source_changes() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let mut q = s.list::<Question>("question").unwrap().remove(0);
    q.answer = "Ma correction manuelle".into();
    s.put("question", &q.id, &q, true).unwrap();
    d.content_hash = "new-revision".into();
    d.text.push_str("\nCondition vérifiée.");
    sync(&s, &Mock::new(), vec![d]).await.unwrap();
    assert_eq!(
        s.list::<Question>("question").unwrap()[0].answer,
        "Ma correction manuelle"
    );
    assert_eq!(
        s.list::<Question>("question").unwrap()[0].status,
        Status::Stale
    );
    assert_eq!(s.list::<Question>("candidate-question").unwrap().len(), 1);
}
#[tokio::test]
async fn simultaneous_runs_are_rejected() {
    let (_t, s) = setup();
    let guard = s.lock().unwrap();
    assert!(
        sync(&s, &Mock::new(), vec![documents().remove(0)])
            .await
            .is_err()
    );
    drop(guard);
    assert!(
        sync(&s, &Mock::new(), vec![documents().remove(0)])
            .await
            .is_ok()
    );
}
#[tokio::test]
async fn classification_changes_are_detected_without_text_change() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    d.level = None;
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    assert!(s.list::<Question>("question").unwrap().is_empty());
    d.level = Some(1);
    assert_eq!(sync(&s, &Mock::new(), vec![d]).await.unwrap().changed, 1);
    assert_eq!(s.list::<Question>("question").unwrap().len(), 1);
}
#[tokio::test]
async fn deleted_source_invalidates_existing_questions() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    d.deleted = true;
    sync(&s, &Mock::new(), vec![d]).await.unwrap();
    assert_eq!(
        s.list::<Question>("question").unwrap()[0].status,
        Status::Stale
    );
}
#[tokio::test]
async fn fictional_premiere_end_to_end() {
    let (t, s) = setup();
    sync(&s, &Mock::new(), documents()).await.unwrap();
    let snapshot = s.snapshot().unwrap();
    assert_eq!(
        (
            snapshot.documents.len(),
            snapshot.fiches.len(),
            snapshot.questions.len()
        ),
        (4, 4, 4)
    );
    for f in &snapshot.fiches {
        let out = study_engine::export::export(f, &t.path().join("exports"), false).unwrap();
        assert!(out.join("fiche.md").exists());
        assert!(out.join("fiche.typ").exists());
        assert!(out.join("fiche.tex").exists());
    }
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    let linux: Snapshot = serde_json::from_slice(&bytes).unwrap();
    let web: Snapshot = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        serde_json::to_value(linux.questions).unwrap(),
        serde_json::to_value(web.questions).unwrap()
    );
}
#[tokio::test]
async fn failed_regeneration_immediately_withdraws_old_training_material() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    s.accept("document", &d.id, Status::Accepted).unwrap();
    let q = s.list::<Question>("question").unwrap().remove(0);
    s.accept("question", &q.id, Status::Accepted).unwrap();
    assert_eq!(training_questions(&s.snapshot().unwrap()).len(), 1);
    d.content_hash = "changed".into();
    d.text.push_str("\nChangement de la source.");
    let mut failing = Mock::new();
    failing.fail_at = 0;
    assert!(sync(&s, &failing, vec![d.clone()]).await.is_err());
    assert!(training_questions(&s.snapshot().unwrap()).is_empty());
    assert_eq!(
        s.get::<Document>("document", &d.id)
            .unwrap()
            .unwrap()
            .content_hash,
        "changed"
    );
}
#[tokio::test]
async fn manual_document_classification_survives_daily_scan() {
    let (_t, s) = setup();
    let d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let mut corrected = d.clone();
    corrected.chapter = "Mon chapitre corrigé".into();
    corrected.level = None;
    s.put("document", &d.id, &corrected, true).unwrap();
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let saved = s.get::<Document>("document", &d.id).unwrap().unwrap();
    assert_eq!(saved.chapter, "Mon chapitre corrigé");
    assert_eq!(saved.level, None);
    assert_eq!(s.list::<Document>("candidate-document").unwrap().len(), 1);
}
#[tokio::test]
async fn obsolete_quotation_cannot_be_accepted_again() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let q = s.list::<Question>("question").unwrap().remove(0);
    d.text = "Source corrigée intégralement.".into();
    d.content_hash = hash(&d.text);
    d.status = Status::Accepted;
    s.put("document", &d.id, &d, true).unwrap();
    assert!(s.accept("question", &q.id, Status::Accepted).is_err());
}
#[test]
fn derived_attachment_accepts_explicit_primary_references() {
    let (t, s) = setup();
    let p = t.path().join("derived.txt");
    std::fs::write(&p, "Fiche dérivée fictive").unwrap();
    let refs = vec![Citation {
        document_id: "demo-maths".into(),
        page: 1,
        quote: "Original fictif".into(),
        region: None,
    }];
    let doc = study_engine::ingestion::import_with_refs(
        &s,
        &p,
        "Mathématiques",
        "Suites",
        DocType::Fiche,
        Some(2),
        "generated",
        None,
        &refs,
    )
    .unwrap();
    assert_eq!(doc.sources, refs);
}
#[tokio::test]
async fn failure_in_first_document_still_withdraws_later_deleted_source() {
    let (_t, s) = setup();
    let mut docs = documents()[..2].to_vec();
    sync(&s, &Mock::new(), docs.clone()).await.unwrap();
    for d in &docs {
        s.accept("document", &d.id, Status::Accepted).unwrap();
    }
    for q in s.list::<Question>("question").unwrap() {
        s.accept("question", &q.id, Status::Accepted).unwrap();
    }
    docs[0].content_hash = "new-first".into();
    docs[0].text.push_str("\nSource modifiée.");
    docs[1].deleted = true;
    let mut failing = Mock::new();
    failing.fail_at = 0;
    assert!(sync(&s, &failing, docs.clone()).await.is_err());
    assert!(training_questions(&s.snapshot().unwrap()).is_empty());
    assert!(
        s.get::<Document>("document", &docs[1].id)
            .unwrap()
            .unwrap()
            .deleted
    );
}
#[tokio::test]
async fn regenerated_candidate_does_not_inherit_obsolete_quotes() {
    let (_t, s) = setup();
    let mut d = documents().remove(0);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    let old = s.list::<Question>("question").unwrap().remove(0);
    d.text = "Suite arithmétique : formulation corrigée.\nCalcul détaillé fictif.".into();
    d.content_hash = hash(&d.text);
    sync(&s, &Mock::new(), vec![d.clone()]).await.unwrap();
    s.accept("document", &d.id, Status::Accepted).unwrap();
    let q = s.get::<Question>("question", &old.id).unwrap().unwrap();
    assert_eq!(q.sources.len(), 1);
    s.accept("question", &q.id, Status::Accepted).unwrap();
}
#[tokio::test]
async fn accepting_candidate_keeps_previous_manual_correction_in_history() {
    let (_t, s) = setup();
    sync(&s, &Mock::new(), vec![documents().remove(0)])
        .await
        .unwrap();
    let mut q = s.list::<Question>("question").unwrap().remove(0);
    q.answer = "Ma réponse corrigée".into();
    s.put("question", &q.id, &q, true).unwrap();
    let mut candidate = q.clone();
    candidate.answer = "Nouvelle proposition à vérifier".into();
    s.put("candidate-question", &q.id, &candidate, false)
        .unwrap();
    s.apply_candidate("question", &q.id, true).unwrap();
    assert_eq!(
        s.get::<Question>("question", &q.id)
            .unwrap()
            .unwrap()
            .answer,
        candidate.answer
    );
    assert_eq!(
        s.list::<Question>("history-question").unwrap()[0].answer,
        "Ma réponse corrigée"
    );
    assert_eq!(
        s.get::<Question>("question", &q.id)
            .unwrap()
            .unwrap()
            .status,
        Status::Pending
    );
}

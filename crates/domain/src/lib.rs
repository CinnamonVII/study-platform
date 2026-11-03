use chrono::{DateTime, Duration, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn id(parts: &[&str]) -> String {
    hash(serde_json::to_vec(parts).unwrap())[..24].to_owned()
}
pub fn now() -> String {
    Utc::now().to_rfc3339()
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Accepted,
    Rejected,
    Stale,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    Cours,
    Devoir,
    Exercice,
    Correction,
    Fiche,
    Methode,
    Annale,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub document_id: String,
    pub page: u32,
    pub quote: String,
    pub region: Option<[f32; 4]>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub kind: DocType,
    pub level: Option<u8>,
    pub provenance_reason: String,
    pub subject: String,
    pub chapter: String,
    pub modified_at: String,
    pub content_hash: String,
    pub status: Status,
    pub sources: Vec<Citation>,
    pub text: String,
    pub original: String,
    pub private: bool,
    pub deleted: bool,
    pub uncertainties: Vec<String>,
    #[serde(default)]
    pub transcription: Option<Transcription>,
}
impl Document {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() || self.subject.trim().is_empty() || self.chapter.trim().is_empty() {
            return Err("Identité, matière et chapitre obligatoires".into());
        }
        if !matches!(self.level, None | Some(1) | Some(2)) {
            return Err("Provenance : 1, 2 ou non classée".into());
        }
        if self.level == Some(1) && self.provenance_reason.trim().is_empty() {
            return Err("Niveau 1 : justification manuelle obligatoire".into());
        }
        if self.level == Some(2) && self.sources.is_empty() {
            return Err("Niveau 2 : références obligatoires".into());
        }
        if self.provenance_reason == "generated" && self.level != Some(2) {
            return Err("Une sortie IA reste de niveau 2".into());
        }
        if self.status == Status::Accepted && !self.uncertainties.is_empty() {
            return Err("Corrigez les incertitudes avant validation".into());
        }
        Ok(())
    }
    pub fn dependency_hash(&self) -> String {
        let mut d = self.clone();
        d.modified_at.clear();
        if d.level != Some(2) && d.status == Status::Accepted {
            d.status = Status::Pending;
        }
        hash(serde_json::to_vec(&d).unwrap())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SectionKind {
    Definition,
    Explanation,
    Method,
    Comparison,
    Timeline,
    Mechanism,
    Alert,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    pub kind: SectionKind,
    pub lines: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fiche {
    pub id: String,
    pub title: String,
    pub subject: String,
    pub chapter: String,
    pub level: u8,
    pub sections: Vec<Section>,
    pub essentials: Vec<String>,
    pub recall: Vec<String>,
    pub sources: Vec<Citation>,
    pub status: Status,
    pub notion_url: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    Short,
    Mcq,
    Open,
    Math,
    Method,
    Flashcard,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Question {
    pub id: String,
    pub subject: String,
    pub chapter: String,
    pub kind: QuestionType,
    pub cognitive: String,
    pub prompt: String,
    pub answer: String,
    pub explanation: String,
    pub choices: Vec<String>,
    pub rubric: Vec<String>,
    pub difficulty: u8,
    pub sources: Vec<Citation>,
    pub status: Status,
    pub created_at: String,
    pub updated_at: String,
    pub generator_version: String,
    pub fiche_ids: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Generated {
    pub fiche: Fiche,
    pub questions: Vec<Question>,
    pub uncertainties: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transcription {
    pub text: String,
    pub uncertainties: Vec<String>,
    pub citations: Vec<Citation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, Default)]
pub struct Snapshot {
    pub schema_version: String,
    pub documents: Vec<Document>,
    pub fiches: Vec<Fiche>,
    pub questions: Vec<Question>,
    pub runs: Vec<Run>,
    pub progress: Vec<Progress>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Run {
    pub id: i64,
    pub started: String,
    pub status: String,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Progress {
    pub question_id: String,
    pub repetitions: u32,
    pub ease: f64,
    pub interval: u32,
    pub due: String,
}
impl Progress {
    pub fn new(question_id: String) -> Self {
        Self {
            question_id,
            repetitions: 0,
            ease: 2.5,
            interval: 0,
            due: now(),
        }
    }
    pub fn review(&mut self, quality: u8, at: DateTime<Utc>) -> Result<(), String> {
        if quality > 5 {
            return Err("Note attendue : 0 à 5".into());
        }
        if quality < 3 {
            self.repetitions = 0;
            self.interval = 1;
        } else {
            self.interval = match self.repetitions {
                0 => 1,
                1 => 6,
                _ => (self.interval as f64 * self.ease).round().max(1.) as u32,
            };
            self.repetitions += 1;
        }
        let d = (5 - quality) as f64;
        self.ease = (self.ease + 0.1 - d * (0.08 + d * 0.02)).max(1.3);
        self.due = (at + Duration::days(self.interval as i64)).to_rfc3339();
        Ok(())
    }
}
pub fn training_questions(s: &Snapshot) -> Vec<&Question> {
    s.questions
        .iter()
        .filter(|q| {
            q.status == Status::Accepted
                && q.sources.iter().all(|c| {
                    s.documents.iter().any(|d| {
                        d.id == c.document_id
                            && d.status == Status::Accepted
                            && !d.deleted
                            && d.uncertainties.is_empty()
                    })
                })
        })
        .collect()
}
pub fn similar(a: &str, b: &str) -> bool {
    let words = |s: &str| {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>()
    };
    let a = words(a);
    let b = words(b);
    let union = a.union(&b).count();
    union > 0 && a.intersection(&b).count() as f64 / union as f64 >= 0.9
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sm2_uses_one_six_then_ease_intervals() {
        let mut p = Progress::new("q".into());
        for expected in [1, 6, 16] {
            p.review(5, Utc::now()).unwrap();
            assert_eq!(p.interval, expected);
        }
        p.review(1, Utc::now()).unwrap();
        assert_eq!((p.repetitions, p.interval), (0, 1));
        assert!(p.ease >= 1.3);
    }
    #[test]
    fn invalid_quality_is_rejected() {
        assert!(Progress::new("q".into()).review(6, Utc::now()).is_err());
    }
    #[test]
    fn generated_content_cannot_be_level_one() {
        let mut d: Vec<Document> =
            serde_json::from_str(include_str!("../../../fixtures/demo/documents.json")).unwrap();
        d[0].provenance_reason = "generated".into();
        assert!(d[0].validate().is_err());
    }
}
impl Question {
    pub fn validate(&self) -> Result<(), String> {
        if self.prompt.trim().is_empty()
            || self.answer.trim().is_empty()
            || self.explanation.trim().is_empty()
            || self.sources.is_empty()
            || !(1..=5).contains(&self.difficulty)
        {
            return Err(
                "Énoncé, correction, explication, sources et difficulté valides requis".into(),
            );
        }
        if ![
            "memorisation",
            "comprehension",
            "application",
            "raisonnement",
        ]
        .contains(&self.cognitive.as_str())
        {
            return Err("Niveau cognitif inconnu".into());
        }
        if self.kind == QuestionType::Mcq
            && (self.choices.len() < 2
                || self.choices.iter().filter(|c| *c == &self.answer).count() != 1)
        {
            return Err("QCM : choix distincts et bonne réponse exacte requis".into());
        }
        if matches!(self.kind, QuestionType::Open | QuestionType::Method) && self.rubric.is_empty()
        {
            return Err("Grille d’évaluation requise".into());
        }
        Ok(())
    }
}
impl Fiche {
    pub fn validate(&self) -> Result<(), String> {
        if self.level != 2
            || self.title.trim().is_empty()
            || self.sections.is_empty()
            || self.essentials.is_empty()
            || self.recall.is_empty()
            || self.sources.is_empty()
        {
            Err("Fiche incomplète : titre, parties, synthèse, rappel et sources requis".into())
        } else {
            Ok(())
        }
    }
}

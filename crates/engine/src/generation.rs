use anyhow::{Result, bail};
use async_trait::async_trait;
use base64::Engine;
use serde_json::{Value, json};
use study_domain::*;
#[async_trait]
pub trait Generator: Send + Sync {
    async fn transcribe(&self, doc: &Document) -> Result<Transcription>;
    async fn generate(&self, doc: &Document) -> Result<Generated>;
}
pub struct OpenAi {
    client: reqwest::Client,
    key: String,
    model: String,
}
impl OpenAi {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()?,
            key: std::env::var("OPENAI_API_KEY")?,
            model: std::env::var("OPENAI_MODEL")?,
        })
    }
    async fn parse<T: serde::de::DeserializeOwned + schemars::JsonSchema>(
        &self,
        prompt: &str,
        content: Value,
    ) -> Result<T> {
        let schema = schemars::schema_for!(T);
        let response=self.client.post("https://api.openai.com/v1/responses").bearer_auth(&self.key).json(&json!({"model":self.model,"store":false,"input":[{"role":"system","content":prompt},{"role":"user","content":content}],"text":{"format":{"type":"json_schema","name":"study_output","strict":false,"schema":schema}}})).send().await?;
        if !response.status().is_success() {
            bail!("API modèle : HTTP {}", response.status())
        }
        let v: Value = response.json().await?;
        if v["status"] != "completed" {
            bail!("Réponse IA incomplète")
        }
        let text = v["output"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|v| v["content"].as_array().into_iter().flatten())
            .find_map(|v| v["text"].as_str())
            .ok_or_else(|| anyhow::anyhow!("Réponse refusée ou vide"))?;
        Ok(serde_json::from_str(text)?)
    }
}
#[async_trait]
impl Generator for OpenAi {
    async fn transcribe(&self, doc: &Document) -> Result<Transcription> {
        let bytes = std::fs::read(&doc.original)?;
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        let item = if doc.original.to_lowercase().ends_with(".pdf") {
            json!({"type":"input_file","filename":"source.pdf","file_data":format!("data:application/pdf;base64,{data}")})
        } else {
            let mime = if doc.original.to_lowercase().ends_with(".png") {
                "image/png"
            } else {
                "image/jpeg"
            };
            json!({"type":"input_image","image_url":format!("data:{mime};base64,{data}")})
        };
        self.parse(
            include_str!("../../../prompts/transcribe_manuscript.md"),
            json!([{"type":"input_text","text":format!("Identifiant source : {}",doc.id)},item]),
        )
        .await
    }
    async fn generate(&self, doc: &Document) -> Result<Generated> {
        self.parse(
            &format!(
                "{}\n{}\n{}",
                include_str!("../../../prompts/generate_fiche.md"),
                include_str!("../../../prompts/generate_questions.md"),
                include_str!("../../../prompts/presentation.md")
            ),
            json!(serde_json::to_string(doc)?),
        )
        .await
    }
}
pub struct FixtureGenerator;
#[async_trait]
impl Generator for FixtureGenerator {
    async fn transcribe(&self, doc: &Document) -> Result<Transcription> {
        Ok(Transcription {
            text: doc.text.clone(),
            uncertainties: doc.uncertainties.clone(),
            citations: vec![],
        })
    }
    async fn generate(&self, d: &Document) -> Result<Generated> {
        let lines = d
            .text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if lines.is_empty() {
            bail!("Source vide")
        }
        let sources = if d.level == Some(2) {
            d.sources.clone()
        } else {
            vec![Citation {
                document_id: d.id.clone(),
                page: 1,
                quote: lines[0].clone(),
                region: None,
            }]
        };
        let f = Fiche {
            id: id(&[&d.id, "fiche"]),
            title: d.chapter.clone(),
            subject: d.subject.clone(),
            chapter: d.chapter.clone(),
            level: 2,
            sections: vec![Section {
                title: "Notions et étapes".into(),
                kind: SectionKind::Method,
                lines: lines.clone(),
            }],
            essentials: lines.iter().take(2).cloned().collect(),
            recall: vec!["Expliquez les notions sans regarder le cours.".into()],
            sources: sources.clone(),
            status: Status::Pending,
            notion_url: String::new(),
        };
        let q = Question {
            id: id(&[&d.id, "question"]),
            subject: d.subject.clone(),
            chapter: d.chapter.clone(),
            kind: QuestionType::Short,
            cognitive: "comprehension".into(),
            prompt: format!("Expliquez : {}", lines[0].split(':').next().unwrap()),
            answer: lines.join("\n"),
            explanation: "Correction extraite de la fixture ; aucune génération IA réelle.".into(),
            choices: vec![],
            rubric: vec![],
            difficulty: 2,
            sources,
            status: Status::Pending,
            created_at: now(),
            updated_at: now(),
            generator_version: "fixture-1".into(),
            fiche_ids: vec![f.id.clone()],
        };
        Ok(Generated {
            fiche: f,
            questions: vec![q],
            uncertainties: d.uncertainties.clone(),
        })
    }
}

mod app;
mod components;
mod dashboard;
mod documents;
mod editor;
mod fiches;
mod practice;
mod sync;
use components::*;
use editor::edit_form;
use eframe::egui::{self, Color32, RichText};
use fiches::fiche_view;
use std::sync::{Arc, Mutex};
use study_domain::*;
const INK: Color32 = Color32::from_rgb(35, 54, 80);
const MUTED: Color32 = Color32::from_rgb(80, 96, 120);
const ACTION: Color32 = Color32::from_rgb(33, 76, 112);
const RED: Color32 = Color32::from_rgb(163, 50, 56);
const GREEN: Color32 = Color32::from_rgb(40, 120, 107);
const PAPER: Color32 = Color32::from_rgb(255, 253, 246);
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Today,
    Documents,
    Fiches,
    Practice,
    Sync,
}
#[derive(Clone)]
enum Reply {
    Snapshot(Snapshot),
    Candidates(serde_json::Value),
    Done(String),
    Pdf(Vec<u8>, String, String),
    Error(String),
}
pub struct StudyApp {
    data: Snapshot,
    theme_ready: bool,
    candidates: serde_json::Value,
    tab: Tab,
    demo: bool,
    endpoint: String,
    token: String,
    notice: String,
    busy: bool,
    replies: Arc<Mutex<Vec<Reply>>>,
    search: String,
    subject: String,
    kind_filter: String,
    level_filter: String,
    answer: String,
    revealed: bool,
    index: usize,
    editor: Option<(String, String, String)>,
    editor_error: String,
    issue: String,
}
impl StudyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme(&cc.egui_ctx);
        Self {
            data: serde_json::from_str(include_str!("../../../fixtures/demo/snapshot.json"))
                .expect("fixture versionnée valide"),
            candidates: serde_json::Value::Null,
            theme_ready: false,
            tab: Tab::Today,
            demo: true,
            endpoint: String::new(),
            token: String::new(),
            notice:
                "Démonstration : quatre documents fictifs. Vos notes privées ne sont pas chargées."
                    .into(),
            busy: false,
            replies: Default::default(),
            search: String::new(),
            subject: String::new(),
            kind_filter: String::new(),
            level_filter: String::new(),
            answer: String::new(),
            revealed: false,
            index: 0,
            editor: None,
            editor_error: String::new(),
            issue: String::new(),
        }
    }
    fn request(
        &mut self,
        ctx: &egui::Context,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) {
        if self.busy {
            return;
        }
        if self.token.len() < 32
            || !(self.endpoint.starts_with("https://")
                || self.endpoint.starts_with("http://127.0.0.1:")
                || self.endpoint.starts_with("http://localhost:"))
        {
            self.notice="Renseignez une adresse HTTPS (ou localhost) et le jeton privé dans Synchronisation.".into();
            self.tab = Tab::Sync;
            return;
        }
        let mut req = ehttp::Request::get(format!("{}{path}", self.endpoint.trim_end_matches('/')));
        req.method = method.into();
        req.headers
            .insert("Authorization", format!("Bearer {}", self.token));
        if let Some(b) = body {
            req.body = serde_json::to_vec(&b).unwrap();
            req.headers.insert("Content-Type", "application/json");
        }
        self.busy = true;
        let inbox = self.replies.clone();
        let ctx = ctx.clone();
        let is_snapshot = path == "/snapshot";
        let is_pdf = path.starts_with("/pdf/") || path.starts_with("/original/");
        let is_candidates = path == "/candidates";
        ehttp::fetch(req, move |r| {
            let reply = match r {
                Ok(r) if r.ok => {
                    if is_snapshot {
                        match serde_json::from_slice::<Snapshot>(&r.bytes) {
                            Ok(s) if s.schema_version == "1.0" => Reply::Snapshot(s),
                            _ => Reply::Error(
                                "Format des données incompatible : version 1.0 attendue.".into(),
                            ),
                        }
                    } else if is_candidates {
                        match serde_json::from_slice(&r.bytes) {
                            Ok(v) => Reply::Candidates(v),
                            Err(e) => Reply::Error(e.to_string()),
                        }
                    } else if is_pdf {
                        Reply::Pdf(
                            r.bytes,
                            r.headers
                                .get("content-disposition")
                                .and_then(|v| v.split("filename=").nth(1))
                                .unwrap_or("fiche-revision.pdf")
                                .to_owned(),
                            r.headers
                                .get("content-type")
                                .unwrap_or("application/octet-stream")
                                .to_owned(),
                        )
                    } else {
                        Reply::Done(if r.bytes.is_empty() {
                            "Modification enregistrée. Actualisez les données pour la consulter."
                                .into()
                        } else {
                            String::from_utf8_lossy(&r.bytes).into_owned()
                        })
                    }
                }
                Ok(r) => Reply::Error(format!(
                    "Serveur : HTTP {}. {}",
                    r.status,
                    r.text()
                        .unwrap_or("Vérifiez la connexion et les autorisations.")
                )),
                Err(e) => Reply::Error(format!("Connexion impossible : {e}")),
            };
            inbox.lock().unwrap().push(reply);
            ctx.request_repaint();
        });
    }
    fn process_replies(&mut self) {
        let replies = std::mem::take(&mut *self.replies.lock().unwrap());
        for r in replies {
            self.busy = false;
            match r {
                Reply::Snapshot(s) => {
                    self.data = s;
                    self.demo = false;
                    self.notice = "Données privées actualisées.".into();
                }
                Reply::Candidates(v) => {
                    self.candidates = v;
                    self.notice =
                        "Propositions chargées. Comparez-les avant de remplacer une correction."
                            .into();
                }
                Reply::Done(s) => self.notice = s,
                Reply::Error(s) => self.notice = s,
                Reply::Pdf(bytes, filename, mime) => {
                    self.notice = match download_pdf(bytes, &filename, &mime) {
                        Ok(()) => "PDF téléchargé.".into(),
                        Err(e) => format!("Enregistrement du PDF impossible : {e}"),
                    };
                }
            }
        }
    }
    fn matches(&self, subject: &str, text: &str) -> bool {
        (self.subject.is_empty() || self.subject == subject)
            && text.to_lowercase().contains(&self.search.to_lowercase())
    }
    fn next(&mut self, len: usize) {
        self.index = (self.index + 1) % len;
        self.revealed = false;
        self.answer.clear();
        self.issue.clear();
    }
    fn edit(&mut self, kind: &str, id: &str, value: &impl serde::Serialize) {
        self.editor = Some((
            kind.into(),
            id.into(),
            serde_json::to_string_pretty(value).unwrap(),
        ));
        self.editor_error.clear();
    }
    fn validate(&mut self, ctx: &egui::Context, kind: &str, id: &str, value: Status) {
        self.request(
            ctx,
            "POST",
            "/validate",
            Some(serde_json::json!({"kind":kind,"id":id,"status":value})),
        );
    }
}

fn chrono_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
#[cfg(not(target_arch = "wasm32"))]
fn download_pdf(bytes: Vec<u8>, filename: &str, _mime: &str) -> Result<(), String> {
    let path = std::env::var("STUDY_DOWNLOAD_PDF").unwrap_or_else(|_| {
        filename
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
            .collect()
    });
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}
#[cfg(target_arch = "wasm32")]
fn download_pdf(bytes: Vec<u8>, filename: &str, _mime: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let array = js_sys::Uint8Array::from(bytes.as_slice());
    let parts = js_sys::Array::new();
    parts.push(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(_mime);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|_| "Création PDF impossible")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "URL PDF impossible")?;
    let doc = web_sys::window().unwrap().document().unwrap();
    let a = doc
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

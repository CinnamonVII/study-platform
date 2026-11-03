use crate::Store;
use anyhow::{Result, bail};
use std::{fs, io::Read, path::Path};
use study_domain::*;
pub fn import(
    store: &Store,
    path: &Path,
    subject: &str,
    chapter: &str,
    kind: DocType,
    level: Option<u8>,
    reason: &str,
    existing_id: Option<&str>,
) -> Result<Document> {
    import_with_refs(
        store,
        path,
        subject,
        chapter,
        kind,
        level,
        reason,
        existing_id,
        &[],
    )
}
pub fn import_with_refs(
    store: &Store,
    path: &Path,
    subject: &str,
    chapter: &str,
    kind: DocType,
    level: Option<u8>,
    reason: &str,
    existing_id: Option<&str>,
    references: &[Citation],
) -> Result<Document> {
    let raw = fs::read(path)?;
    if raw.len() > 20 * 1024 * 1024 {
        bail!("Document limité à 20 Mo")
    }
    let content_hash = hash(&raw);
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Nom absent"))?;
    let original = store
        .root
        .join("originals")
        .join(&content_hash)
        .join(filename);
    fs::create_dir_all(original.parent().unwrap())?;
    if !original.exists() {
        fs::write(&original, &raw)?;
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let text = match ext.as_str() {
        "md" | "txt" => String::from_utf8(raw)?,
        "docx" => {
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(raw))?;
            let mut xml = String::new();
            z.by_name("word/document.xml")?
                .take(20 * 1024 * 1024)
                .read_to_string(&mut xml)?;
            let mut reader = quick_xml::Reader::from_str(&xml);
            let mut text = String::new();
            loop {
                match reader.read_event()? {
                    quick_xml::events::Event::Text(t) => {
                        text.push_str(&quick_xml::escape::unescape(&t.decode()?)?);
                    }
                    quick_xml::events::Event::End(e) if e.name().as_ref() == b"w:p" => {
                        text.push('\n')
                    }
                    quick_xml::events::Event::Eof => break,
                    _ => {}
                }
            }
            text
        }
        "pdf" => {
            let pdf = lopdf::Document::load(path)?;
            let pages = pdf.get_pages().keys().copied().collect::<Vec<_>>();
            pdf.extract_text(&pages).unwrap_or_default()
        }
        "png" | "jpg" | "jpeg" => String::new(),
        _ => bail!("Format non pris en charge"),
    };
    let d = Document {
        id: existing_id.map(str::to_owned).unwrap_or_else(|| {
            id(&[&path
                .canonicalize()
                .unwrap_or(path.to_owned())
                .to_string_lossy()])
        }),
        title: path.file_stem().unwrap().to_string_lossy().into(),
        kind,
        level,
        provenance_reason: reason.into(),
        subject: subject.into(),
        chapter: chapter.into(),
        modified_at: now(),
        content_hash,
        status: Status::Pending,
        sources: references.to_vec(),
        text,
        original: original.to_string_lossy().into(),
        private: true,
        deleted: false,
        uncertainties: vec![],
        transcription: None,
    };
    d.validate().map_err(anyhow::Error::msg)?;
    Ok(d)
}

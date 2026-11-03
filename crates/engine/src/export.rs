use anyhow::{Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use study_domain::*;
pub fn markdown(f: &Fiche) -> String {
    let mut out = format!(
        "# {}\n\n{} · {}\n\nStatut : {:?} · Provenance : niveau 2\n",
        f.title, f.subject, f.chapter, f.status
    );
    for s in &f.sections {
        out.push_str(&format!("\n## {}\n\n", s.title));
        for l in &s.lines {
            out.push_str(&format!("- {l}\n"));
        }
    }
    out.push_str("\n## À retenir\n\n");
    for l in &f.essentials {
        out.push_str(&format!("- {l}\n"));
    }
    out.push_str("\n## Rappel actif\n\n");
    for l in &f.recall {
        out.push_str(&format!("- {l}\n"));
    }
    out.push_str("\n## Sources\n\n");
    for s in &f.sources {
        out.push_str(&format!(
            "- {} · p. {} : « {} »\n",
            s.document_id, s.page, s.quote
        ));
    }
    out
}
fn txt(s: &str) -> String {
    format!("#text({})", serde_json::to_string(s).unwrap())
}
pub fn typst(f: &Fiche) -> String {
    let mut t = String::from(
        r##"#set page(paper: "a4", margin: 17mm, fill: rgb("fffdf6"), numbering: "1", background: {
 for x in range(0, 43) { place(top + left, dx: x * 5mm, line(end: (0pt, 297mm), stroke: 0.2pt + rgb("e6e9e6"))) }
 for y in range(0, 60) { place(top + left, dy: y * 5mm, line(end: (210mm, 0pt), stroke: 0.2pt + rgb("e6e9e6"))) }
})
#set text(font: "DejaVu Sans", size: 11pt, fill: rgb("304b72"), lang: "fr")
#set par(leading: 0.65em)
#set heading(numbering: none)
#show heading.where(level: 1): set text(size: 16pt, fill: rgb("bd4d50"))
#show heading.where(level: 2): set text(size: 14pt, fill: rgb("28786b"))
"##,
    );
    t.push_str(&format!(
        "#text(size: 24pt, weight: \"bold\", fill: rgb(\"bd4d50\"), {})\n\n{}\n\n{}\n",
        serde_json::to_string(&f.title).unwrap(),
        txt(&format!("{} · {}", f.subject, f.chapter)),
        txt(&format!(
            "Fiche de niveau 2 · {}",
            match f.status {
                Status::Accepted => "Validée",
                Status::Pending => "À valider",
                Status::Rejected => "Rejetée",
                Status::Stale => "À revérifier",
            }
        ))
    ));
    for s in &f.sections {
        t.push_str(&format!(
            "\n#heading(level: 1, {})\n",
            serde_json::to_string(&s.title).unwrap()
        ));
        let color = match s.kind {
            SectionKind::Definition => "28786b",
            SectionKind::Alert => "bd4d50",
            _ => "304b72",
        };
        if matches!(s.kind, SectionKind::Comparison) {
            t.push_str(
                "#table(columns: (1fr, 1fr), stroke: 0.5pt + rgb(\"eed17b\"), inset: 7pt,\n",
            );
            for line in &s.lines {
                let (a, b) = line.split_once('|').unwrap_or((line.as_str(), ""));
                t.push_str(&format!("[{}], [{}],\n", txt(a.trim()), txt(b.trim())));
            }
            t.push_str(")\n");
        } else {
            for (i, l) in s.lines.iter().enumerate() {
                let prefix = if matches!(s.kind, SectionKind::Method | SectionKind::Timeline) {
                    format!("{}. ", i + 1)
                } else {
                    "• ".into()
                };
                t.push_str(&format!(
                    "#block(inset: (left: 6pt), text(fill: rgb(\"{color}\"), {}))\n",
                    serde_json::to_string(&(prefix + l)).unwrap()
                ));
                if matches!(s.kind, SectionKind::Mechanism) && i + 1 < s.lines.len() {
                    t.push_str("#align(center, text(fill: rgb(\"78608d\"))[↓])\n");
                }
            }
        }
    }
    t.push_str("\n#heading(level: 1)[À retenir]\n#block(fill: rgb(\"fff1bc\"), inset: 10pt, radius: 2pt)[\n");
    for l in &f.essentials {
        t.push_str(&format!("{}\n\n", txt(l)));
    }
    t.push_str("]\n#heading(level: 1)[Rappel actif]\n");
    for l in &f.recall {
        t.push_str(&format!("{}\n\n", txt(l)));
    }
    t.push_str("#heading(level: 2)[Sources]\n");
    for s in &f.sources {
        t.push_str(&format!(
            "#text(size: 8pt, {})\n\n",
            serde_json::to_string(&format!("{} · p. {} : {}", s.document_id, s.page, s.quote))
                .unwrap()
        ));
    }
    t
}
fn tex_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' => "\\textbackslash{}".into(),
            '{' => "\\{".into(),
            '}' => "\\}".into(),
            '$' => "\\$".into(),
            '&' => "\\&".into(),
            '#' => "\\#".into(),
            '%' => "\\%".into(),
            '_' => "\\_".into(),
            '^' => "\\textasciicircum{}".into(),
            '~' => "\\textasciitilde{}".into(),
            _ => c.to_string(),
        })
        .collect()
}
pub fn latex(f: &Fiche) -> String {
    let mut out = format!(
        "\\documentclass[a4paper,11pt]{{article}}\n\\usepackage{{fontspec}}\n\\usepackage[margin=17mm]{{geometry}}\n\\usepackage{{xcolor}}\n\\pagecolor[HTML]{{FFFDF6}}\n\\begin{{document}}\n\\section*{{{}}}\n",
        tex_escape(&f.title)
    );
    for s in &f.sections {
        out.push_str(&format!(
            "\\subsection*{{{}}}\n\\begin{{itemize}}\n",
            tex_escape(&s.title)
        ));
        for l in &s.lines {
            out.push_str(&format!("\\item {}\n", tex_escape(l)));
        }
        out.push_str("\\end{itemize}\n");
    }
    for (title, lines) in [("À retenir", &f.essentials), ("Rappel actif", &f.recall)] {
        out.push_str(&format!("\\subsection*{{{title}}}\n"));
        for l in lines {
            out.push_str(&format!("{}\\par\n", tex_escape(l)));
        }
    }
    out.push_str("\\subsection*{Sources}\n");
    for c in &f.sources {
        out.push_str(&format!(
            "{} p. {} : {}\\par\n",
            tex_escape(&c.document_id),
            c.page,
            tex_escape(&c.quote)
        ));
    }
    out.push_str("\\end{document}\n");
    out
}
pub fn export(f: &Fiche, root: &Path, pdf: bool) -> Result<PathBuf> {
    if f.id.is_empty() || !f.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        bail!("Identifiant export invalide")
    }
    let dir = root.join(&f.id);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("fiche.md"), markdown(f))?;
    fs::write(dir.join("fiche.tex"), latex(f))?;
    fs::write(dir.join("fiche.typ"), typst(f))?;
    if pdf {
        let result = Command::new(std::env::var("TYPST_BIN").unwrap_or_else(|_| "typst".into()))
            .arg("compile")
            .arg("--root")
            .arg(&dir)
            .arg(dir.join("fiche.typ"))
            .arg(dir.join("fiche.pdf"))
            .output()?;
        if !result.status.success() {
            bail!("Export Typst : {}", String::from_utf8_lossy(&result.stderr))
        }
    }
    Ok(dir)
}
pub fn notion_blocks(f: &Fiche) -> Vec<serde_json::Value> {
    use serde_json::json;
    fn rich(text: &str, color: &str) -> Vec<serde_json::Value> {
        text.chars().collect::<Vec<_>>().chunks(1800).map(|s|json!({"type":"text","text":{"content":s.iter().collect::<String>()},"annotations":{"color":color}})).collect()
    }
    fn block(kind: &str, text: &str, color: &str) -> serde_json::Value {
        json!({"object":"block","type":kind,kind:{"rich_text":rich(text,color)}})
    }
    let mut blocks = vec![
        block("heading_1", &f.title, "red"),
        block(
            "paragraph",
            &format!("{} · {} · Niveau 2", f.subject, f.chapter),
            "blue",
        ),
    ];
    for s in &f.sections {
        blocks.push(block("heading_2", &s.title, "red"));
        let color = match s.kind {
            SectionKind::Definition => "green",
            SectionKind::Alert => "red",
            _ => "blue",
        };
        if matches!(s.kind, SectionKind::Comparison) {
            let rows=s.lines.iter().map(|l|{let(a,b)=l.split_once('|').unwrap_or((l,""));json!({"object":"block","type":"table_row","table_row":{"cells":[rich(a.trim(),color),rich(b.trim(),color)]}})}).collect::<Vec<_>>();
            if !rows.is_empty() {
                blocks.push(json!({"object":"block","type":"table","table":{"table_width":2,"has_column_header":false,"has_row_header":false,"children":rows}}));
            }
        } else {
            for (i, line) in s.lines.iter().enumerate() {
                blocks.push(block(
                    if matches!(s.kind, SectionKind::Method | SectionKind::Timeline) {
                        "numbered_list_item"
                    } else {
                        "bulleted_list_item"
                    },
                    line,
                    color,
                ));
                if matches!(s.kind, SectionKind::Mechanism) && i + 1 < s.lines.len() {
                    blocks.push(block("paragraph", "↓", "purple"));
                }
            }
        }
    }
    for (title, lines) in [("À retenir", &f.essentials), ("Rappel actif", &f.recall)] {
        blocks.push(block("heading_2", title, "red"));
        for l in lines {
            blocks.push(block(
                "bulleted_list_item",
                l,
                if title == "À retenir" {
                    "yellow_background"
                } else {
                    "blue"
                },
            ));
        }
    }
    blocks.push(block("heading_2", "Sources", "green"));
    for c in &f.sources {
        blocks.push(block(
            "paragraph",
            &format!("{} · p. {} : {}", c.document_id, c.page, c.quote),
            "default",
        ));
    }
    blocks
}

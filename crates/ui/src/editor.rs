use super::*;
pub(super) fn field(ui: &mut egui::Ui, label: &str, value: &mut String, multiline: bool) {
    ui.label(RichText::new(label).strong());
    if multiline {
        ui.add(
            egui::TextEdit::multiline(value)
                .desired_width(f32::INFINITY)
                .desired_rows(4),
        );
    } else {
        ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY));
    }
}

pub(super) fn lines_field(ui: &mut egui::Ui, label: &str, value: &mut Vec<String>) {
    let mut text = value.join("\n");
    field(ui, label, &mut text, true);
    *value = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_owned)
        .collect();
}

pub(super) fn edit_form(ui: &mut egui::Ui, kind: &str, text: &mut String) {
    match kind {
        "document" => {
            if let Ok(mut d) = serde_json::from_str::<Document>(text) {
                field(ui, "Titre", &mut d.title, false);
                field(ui, "Matière", &mut d.subject, false);
                field(ui, "Chapitre", &mut d.chapter, false);
                ui.label("Type de document");
                egui::ComboBox::from_id_salt("document-kind")
                    .selected_text(format!("{:?}", d.kind))
                    .show_ui(ui, |ui| {
                        for (k, label) in [
                            (DocType::Cours, "Cours"),
                            (DocType::Devoir, "Devoir"),
                            (DocType::Exercice, "Exercice"),
                            (DocType::Correction, "Correction"),
                            (DocType::Fiche, "Fiche"),
                            (DocType::Methode, "Méthode"),
                            (DocType::Annale, "Annale"),
                        ] {
                            ui.selectable_value(&mut d.kind, k, label);
                        }
                    });
                ui.label("Provenance");
                egui::ComboBox::from_id_salt("document-level")
                    .selected_text(match d.level {
                        Some(1) => "Niveau 1 : original",
                        Some(2) => "Niveau 2 : dérivé",
                        _ => "Non classé",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut d.level, None, "Non classé");
                        ui.selectable_value(&mut d.level, Some(1), "Niveau 1 : original");
                        ui.selectable_value(&mut d.level, Some(2), "Niveau 2 : dérivé");
                    });
                field(
                    ui,
                    "Justification de la provenance",
                    &mut d.provenance_reason,
                    false,
                );
                field(ui, "Transcription et formules", &mut d.text, true);
                for u in &d.uncertainties {
                    ui.colored_label(RED, format!("À vérifier : {u}"));
                }
                if !d.uncertainties.is_empty() {
                    let mut checked = false;
                    ui.checkbox(
                        &mut checked,
                        "J’ai vérifié et corrigé tous les passages signalés",
                    );
                    if checked {
                        d.uncertainties.clear();
                    }
                }
                *text = serde_json::to_string(&d).unwrap();
            }
        }
        "fiche" => {
            if let Ok(mut f) = serde_json::from_str::<Fiche>(text) {
                field(ui, "Titre", &mut f.title, false);
                for (i, s) in f.sections.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        field(ui, "Titre de la partie", &mut s.title, false);
                        lines_field(ui, "Idées : une par ligne", &mut s.lines);
                    });
                }
                lines_field(ui, "À retenir", &mut f.essentials);
                lines_field(ui, "Rappel actif", &mut f.recall);
                *text = serde_json::to_string(&f).unwrap();
            }
        }
        "question" => {
            if let Ok(mut q) = serde_json::from_str::<Question>(text) {
                field(ui, "Énoncé", &mut q.prompt, true);
                field(ui, "Réponse et résolution", &mut q.answer, true);
                field(
                    ui,
                    "Explication des étapes et pièges",
                    &mut q.explanation,
                    true,
                );
                if q.kind == QuestionType::Mcq {
                    lines_field(ui, "Choix : une réponse par ligne", &mut q.choices);
                }
                if matches!(q.kind, QuestionType::Open | QuestionType::Method) {
                    lines_field(ui, "Critères d’évaluation", &mut q.rubric);
                }
                *text = serde_json::to_string(&q).unwrap();
            }
        }
        _ => {
            ui.label("Ce contenu ne peut pas être corrigé ici.");
        }
    }
}

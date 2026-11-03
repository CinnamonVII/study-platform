use super::*;
impl StudyApp {
    pub(super) fn filters(&mut self, ui: &mut egui::Ui) {
        ui.label("Rechercher un chapitre, un titre ou une notion");
        ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(f32::INFINITY));
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("subject")
                .selected_text(if self.subject.is_empty() {
                    "Toutes les matières"
                } else {
                    &self.subject
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.subject, String::new(), "Toutes les matières");
                    for s in ["Mathématiques", "SES", "HGGSP", "Français"] {
                        ui.selectable_value(&mut self.subject, s.into(), s);
                    }
                });
        });
        ui.add_space(16.);
    }

    pub(super) fn documents(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        title(
            ui,
            "Documents",
            "L’original reste conservé à côté de sa transcription.",
        );
        self.filters(ui);
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("level")
                .selected_text(if self.level_filter.is_empty() {
                    "Toutes les provenances"
                } else {
                    &self.level_filter
                })
                .show_ui(ui, |ui| {
                    for s in ["", "Niveau 1", "Niveau 2", "Non classé"] {
                        ui.selectable_value(
                            &mut self.level_filter,
                            s.into(),
                            if s.is_empty() {
                                "Toutes les provenances"
                            } else {
                                s
                            },
                        );
                    }
                });
            egui::ComboBox::from_id_salt("kind")
                .selected_text(if self.kind_filter.is_empty() {
                    "Tous les types"
                } else {
                    &self.kind_filter
                })
                .show_ui(ui, |ui| {
                    for s in [
                        "",
                        "Cours",
                        "Devoir",
                        "Exercice",
                        "Correction",
                        "Fiche",
                        "Methode",
                        "Annale",
                    ] {
                        ui.selectable_value(
                            &mut self.kind_filter,
                            s.into(),
                            if s.is_empty() { "Tous les types" } else { s },
                        );
                    }
                });
        });
        ui.add_space(16.);
        let docs = self.data.documents.clone();
        let mut shown = 0;
        for d in docs {
            let provenance = match d.level {
                Some(1) => "Niveau 1",
                Some(2) => "Niveau 2",
                _ => "Non classé",
            };
            if !self.matches(&d.subject, &format!("{} {} {}", d.title, d.chapter, d.text))
                || (!self.level_filter.is_empty() && self.level_filter != provenance)
                || (!self.kind_filter.is_empty() && self.kind_filter != format!("{:?}", d.kind))
            {
                continue;
            }
            shown += 1;
            ui.push_id(&d.id, |ui| {
                egui::CollapsingHeader::new(RichText::new(&d.title).size(19.).strong())
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(format!(
                            "{} · {} · {:?} · {}",
                            d.subject, d.chapter, d.kind, provenance
                        ));
                        status(ui, d.status);
                        ui.label(&d.provenance_reason);
                        ui.add_space(8.);
                        for u in &d.uncertainties {
                            ui.colored_label(RED, format!("À vérifier : {u}"));
                        }
                        ui.label(RichText::new("Transcription").strong());
                        ui.label(&d.text);
                    if let Some(transcription)=&d.transcription{ui.collapsing("Repères de la transcription initiale",|ui|{for c in &transcription.citations{ui.label(format!("Page {} : {}",c.page,c.quote));if let Some(r)=c.region{ui.label(format!("Région source : x {:.2}, y {:.2}, largeur {:.2}, hauteur {:.2}",r[0],r[1],r[2],r[3]));}}});}
                        if d.original.starts_with("https://www.notion.so/") {
                            ui.hyperlink_to("Ouvrir la source Notion", &d.original);
                        } else if !d.original.is_empty() {
                            ui.label("Original immuable conservé sur le serveur privé.");
                        }
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Télécharger l’original").clicked() {
                                self.request(ctx, "GET", &format!("/original/{}", d.id), None);
                            }
                            if ui.button("Corriger / classer").clicked() {
                                self.edit("document", &d.id, &d);
                            }
                            if ui.button("Valider la source").clicked() {
                                self.validate(ctx, "document", &d.id, Status::Accepted);
                            }
                            if ui.button("Rejeter").clicked() {
                                self.validate(ctx, "document", &d.id, Status::Rejected);
                            }
                        });
                    });
            });
            ui.separator();
        }
        if shown == 0 {
            ui.label("Aucun document pour ces filtres. Modifiez la recherche ou importez une source avec la commande d’import documentée.");
        }
    }
}

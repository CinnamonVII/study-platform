use super::*;
impl StudyApp {
    pub(super) fn practice(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        title(
            ui,
            "Entraînement",
            "Cherchez votre réponse avant de découvrir la correction.",
        );
        let mut qs = training_questions(&self.data)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        qs.sort_by_key(|q| {
            self.data
                .progress
                .iter()
                .find(|p| p.question_id == q.id)
                .map(|p| p.due.clone())
                .unwrap_or_default()
        });
        if qs.is_empty() {
            ui.label("Aucune question validée. Vérifiez les sources, puis acceptez les questions dans Synchronisation.");
            return;
        }
        self.index %= qs.len();
        let q = qs[self.index].clone();
        ui.label(format!(
            "Question {} sur {} · {} · {}",
            self.index + 1,
            qs.len(),
            q.subject,
            q.chapter
        ));
        if let Some(p) = self.data.progress.iter().find(|p| p.question_id == q.id) {
            ui.label(format!("Prochaine révision : {}", p.due));
        }
        ui.add_space(16.);
        ui.label(RichText::new(&q.prompt).size(24.).strong().color(INK));
        ui.add_space(16.);
        ui.label("Votre réponse");
        if q.kind == QuestionType::Mcq {
            for choice in &q.choices {
                ui.radio_value(&mut self.answer, choice.clone(), choice);
            }
        } else {
            ui.add(
                egui::TextEdit::multiline(&mut self.answer)
                    .desired_width(f32::INFINITY)
                    .desired_rows(5),
            );
        }
        ui.add_space(12.);
        if !self.revealed && primary(ui, "Voir la correction") {
            self.revealed = true;
        }
        if self.revealed {
            ui.add_space(16.);
            egui::Frame::new()
                .fill(PAPER)
                .inner_margin(20)
                .show(ui, |ui| {
                    ui.heading("Correction");
                    ui.label(&q.answer);
                    ui.add_space(8.);
                    ui.label(&q.explanation);
                    for item in &q.rubric {
                        ui.label(format!("• {item}"));
                    }
                });
            ui.add_space(16.);
            ui.label("Autoévaluation : à quel point aviez-vous retrouvé la réponse ?");
            ui.horizontal_wrapped(|ui| {
                for (score, label) in [
                    (0, "À reprendre"),
                    (3, "Difficile"),
                    (4, "Retrouvé"),
                    (5, "Facile"),
                ] {
                    if ui.button(label).clicked() {
                        if self.demo {
                            let mut p = self
                                .data
                                .progress
                                .iter()
                                .find(|p| p.question_id == q.id)
                                .cloned()
                                .unwrap_or_else(|| Progress::new(q.id.clone()));
                            let _ = p.review(score, chrono_now());
                            self.data.progress.retain(|v| v.question_id != q.id);
                            self.data.progress.push(p);
                            self.notice =
                                "Progression de démonstration enregistrée pour cette session."
                                    .into();
                        } else {
                            self.request(
                                ctx,
                                "POST",
                                "/review",
                                Some(serde_json::json!({"question_id":q.id,"quality":score})),
                            );
                        }
                        self.next(qs.len());
                    }
                }
            });
        }
        ui.add_space(16.);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Question suivante").clicked() {
                self.next(qs.len());
            }
            if ui.button("Corriger cette question").clicked() {
                self.edit("question", &q.id, &q);
            }
        });
        ui.collapsing("Fiche et sources associées", |ui| {
            for f in &self.data.fiches {
                if q.fiche_ids.contains(&f.id) {
                    fiche_view(ui, f);
                }
            }
            for c in &q.sources {
                citation_view(ui, c, &self.data);
            }
        });
        ui.collapsing("Signaler une erreur", |ui| {
            ui.label("Décrivez l’erreur");
            ui.text_edit_multiline(&mut self.issue);
            if ui.button("Envoyer le signalement").clicked() {
                if self.issue.trim().is_empty() {
                    self.notice = "Décrivez l’erreur avant l’envoi.".into();
                } else {
                    self.request(
                        ctx,
                        "POST",
                        "/report",
                        Some(serde_json::json!({"question_id":q.id,"detail":self.issue})),
                    );
                }
            }
        });
    }
}

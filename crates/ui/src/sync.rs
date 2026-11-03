use super::*;
impl StudyApp {
    pub(super) fn sync(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        title(
            ui,
            "Synchronisation",
            "Connectez votre espace privé et vérifiez les nouveautés du matin.",
        );
        ui.label("Adresse du serveur privé");
        ui.add(
            egui::TextEdit::singleline(&mut self.endpoint)
                .hint_text("https://revisions.votre-domaine.fr")
                .desired_width(f32::INFINITY),
        );
        ui.label("Jeton d’accès personnel");
        ui.add(
            egui::TextEdit::singleline(&mut self.token)
                .password(true)
                .desired_width(f32::INFINITY),
        );
        ui.label(
            RichText::new("Le jeton reste en mémoire pendant cette session.")
                .color(MUTED)
                .size(13.),
        );
        ui.add_space(16.);
        ui.horizontal_wrapped(|ui| {
            if primary(ui, "Charger mes données") {
                self.request(ctx, "GET", "/snapshot", None);
            }
            if ui.button("Déconnecter").clicked() {
                self.token.clear();
                self.data =
                    serde_json::from_str(include_str!("../../../fixtures/demo/snapshot.json"))
                        .unwrap();
                self.demo = true;
                self.notice = "Déconnecté. Démonstration fictive affichée.".into();
            }
        });
        ui.add_space(24.);
        ui.heading("Traitement documentaire");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Synchroniser Notion").clicked() {
                self.request(ctx, "POST", "/sync", None);
            }
            if ui.button("Relancer la génération").clicked() {
                self.request(ctx, "POST", "/generate", None);
            }
            if ui.button("Proposer une PR GitHub privée").clicked() {
                self.request(ctx, "POST", "/publish/github", None);
            }
        });
        ui.label("Les nouveaux contenus attendent votre validation. La planification à 7 h doit être activée dans GitHub et ChatGPT.");
        ui.add_space(24.);
        ui.heading("Mises à jour proposées");
        if ui.button("Prévisualiser les propositions").clicked() {
            self.request(ctx, "GET", "/candidates", None);
        }
        for (key, kind) in [
            ("documents", "document"),
            ("fiches", "fiche"),
            ("questions", "question"),
        ] {
            for value in self.candidates[key].as_array().cloned().unwrap_or_default() {
                let id = value["id"].as_str().unwrap_or("").to_owned();
                let label = value["title"]
                    .as_str()
                    .or_else(|| value["prompt"].as_str())
                    .unwrap_or("Proposition")
                    .to_owned();
                ui.push_id(format!("candidate-{id}"), |ui| {
                    ui.collapsing(label, |ui| {
                        if kind == "fiche" {
                            if let Ok(f) = serde_json::from_value::<Fiche>(value.clone()) {
                                fiche_view(ui, &f);
                            }
                        } else if kind == "question" {
                            ui.label(value["answer"].as_str().unwrap_or(""));
                        } else {
                            ui.label(format!(
                                "Matière : {} · chapitre : {} · niveau : {}",
                                value["subject"], value["chapter"], value["level"]
                            ));
                            ui.label(value["text"].as_str().unwrap_or(""));
                        }
                        ui.label("L’ancienne version sera conservée dans l’historique privé.");
                        ui.horizontal_wrapped(|ui| {
                            for (apply, label) in [
                                (true, "Remplacer par cette proposition"),
                                (false, "Écarter cette proposition"),
                            ] {
                                if ui.button(label).clicked() {
                                    self.request(
                                        ctx,
                                        "POST",
                                        "/candidate",
                                        Some(
                                            serde_json::json!({"kind":kind,"id":id,"apply":apply}),
                                        ),
                                    );
                                }
                            }
                        });
                    });
                });
            }
        }
        ui.add_space(24.);
        ui.heading("Questions à vérifier");
        for q in self.data.questions.clone() {
            if q.status == Status::Accepted {
                continue;
            }
            ui.push_id(&q.id, |ui| {
                ui.collapsing(&q.prompt, |ui| {
                    status(ui, q.status);
                    ui.label(&q.answer);
                    ui.label(&q.explanation);
                    for c in &q.sources {
                        citation_view(ui, c, &self.data);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Corriger").clicked() {
                            self.edit("question", &q.id, &q);
                        }
                        if ui.button("Accepter").clicked() {
                            self.validate(ctx, "question", &q.id, Status::Accepted);
                        }
                        if ui.button("Rejeter").clicked() {
                            self.validate(ctx, "question", &q.id, Status::Rejected);
                        }
                    });
                });
            });
        }
        ui.add_space(24.);
        ui.heading("Journal des passages");
        for r in &self.data.runs {
            ui.label(format!("{} · {}", r.started, r.status));
            ui.label(&r.detail);
            ui.separator();
        }
    }
}

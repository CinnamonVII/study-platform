use super::*;
impl StudyApp {
    pub(super) fn today(&mut self, ui: &mut egui::Ui) {
        title(
            ui,
            "Votre séance du jour",
            "Retrouver le fil, comprendre, puis rappeler sans regarder.",
        );
        let questions = training_questions(&self.data);
        let due = questions
            .iter()
            .filter(|q| {
                self.data
                    .progress
                    .iter()
                    .find(|p| p.question_id == q.id)
                    .is_none_or(|p| p.due <= now())
            })
            .count();
        egui::Frame::new()
            .fill(Color32::WHITE)
            .inner_margin(24)
            .corner_radius(8)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(if due > 0 {
                        format!("{due} question(s) à revoir")
                    } else {
                        "Votre prochaine séance est prête".into()
                    })
                    .size(24.)
                    .strong()
                    .color(INK),
                );
                ui.label("Une réponse à la fois. La correction apparaît après votre tentative.");
                ui.add_space(16.);
                if primary(ui, "Commencer la révision") {
                    self.tab = Tab::Practice;
                    self.index = 0;
                    self.revealed = false;
                }
            });
        ui.add_space(24.);
        ui.heading("Vos matières");
        ui.add_space(8.);
        for subject in ["Mathématiques", "SES", "HGGSP", "Français"] {
            let fiches = self
                .data
                .fiches
                .iter()
                .filter(|f| f.subject == subject && f.status == Status::Accepted)
                .count();
            let qs = self
                .data
                .questions
                .iter()
                .filter(|q| q.subject == subject && q.status == Status::Accepted)
                .count();
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(subject).strong().size(19.).color(INK));
                ui.label(format!("{fiches} fiche(s) · {qs} question(s)"));
                if ui.button("Consulter").clicked() {
                    self.subject = subject.into();
                    self.tab = Tab::Fiches;
                }
            });
            ui.add_space(8.);
            ui.separator();
            ui.add_space(8.);
        }
        ui.add_space(16.);
        ui.heading("À vérifier avant de réviser");
        let pending = self
            .data
            .fiches
            .iter()
            .filter(|f| f.status != Status::Accepted)
            .count();
        if pending == 0 {
            ui.label("Aucune fiche en attente. Les prochains documents apparaîtront ici après synchronisation.");
        } else {
            ui.label(format!("{pending} fiche(s) attendent votre lecture."));
            if ui.button("Ouvrir les fiches").clicked() {
                self.subject.clear();
                self.tab = Tab::Fiches;
            }
        }
        ui.add_space(24.);
        ui.heading("Dernier passage du moteur");
        if let Some(run) = self.data.runs.first() {
            ui.label(format!("{} · {}", run.started, run.status));
            ui.label(&run.detail);
        } else {
            ui.label("Aucune synchronisation effectuée.");
        }
    }
}

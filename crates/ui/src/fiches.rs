use super::*;
impl StudyApp {
    pub(super) fn fiches(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        title(
            ui,
            "Fiches de révision",
            "Des notions reliées à leurs sources, toujours vérifiables.",
        );
        self.filters(ui);
        let fiches = self.data.fiches.clone();
        let mut shown = 0;
        for f in fiches {
            if !self.matches(&f.subject, &format!("{} {}", f.title, f.chapter)) {
                continue;
            }
            shown += 1;
            ui.push_id(&f.id, |ui| {
                egui::CollapsingHeader::new(RichText::new(&f.title).size(21.).strong())
                    .default_open(shown == 1)
                    .show(ui, |ui| {
                        fiche_view(ui, &f);
                        ui.add_space(12.);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Corriger la fiche").clicked() {
                                self.edit("fiche", &f.id, &f);
                            }
                            if ui.button("Accepter").clicked() {
                                self.validate(ctx, "fiche", &f.id, Status::Accepted);
                            }
                            if ui.button("Rejeter").clicked() {
                                self.validate(ctx, "fiche", &f.id, Status::Rejected);
                            }
                            if ui.button("Télécharger le PDF").clicked() {
                                self.request(ctx, "GET", &format!("/pdf/{}", f.id), None);
                            }
                            if ui.button("Enregistrer dans Notion").clicked() {
                                self.request(
                                    ctx,
                                    "POST",
                                    &format!("/publish/notion/{}", f.id),
                                    None,
                                );
                            }
                        });
                        if !f.notion_url.is_empty() {
                            ui.hyperlink_to("Ouvrir dans Notion", &f.notion_url);
                        }
                    });
            });
            ui.add_space(16.);
        }
        if shown == 0 {
            ui.label("Aucune fiche pour ces filtres. Les fiches générées apparaîtront après le premier traitement.");
        }
    }
}

pub(super) fn fiche_view(ui: &mut egui::Ui, f: &Fiche) {
    egui::Frame::new()
        .fill(PAPER)
        .inner_margin(24)
        .stroke(egui::Stroke::new(1_f32, Color32::from_rgb(238, 209, 123)))
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width().min(760.));
            ui.label(RichText::new(&f.title).size(26.).strong().color(RED));
            ui.label(format!("{} · {} · Niveau 2", f.subject, f.chapter));
            status(ui, f.status);
            ui.add_space(16.);
            for s in &f.sections {
                ui.label(RichText::new(&s.title).size(21.).strong().color(RED));
                for (i, line) in s.lines.iter().enumerate() {
                    let color = match s.kind {
                        SectionKind::Definition => GREEN,
                        SectionKind::Alert => RED,
                        _ => INK,
                    };
                    let prefix = if matches!(s.kind, SectionKind::Method | SectionKind::Timeline) {
                        format!("{}. ", i + 1)
                    } else {
                        "• ".into()
                    };
                    ui.label(RichText::new(prefix + line).color(color));
                    if matches!(s.kind, SectionKind::Mechanism) && i + 1 < s.lines.len() {
                        ui.colored_label(Color32::from_rgb(120, 96, 141), "↓");
                    }
                }
                ui.add_space(16.);
            }
            ui.label(RichText::new("À retenir").size(21.).strong().color(RED));
            for l in &f.essentials {
                ui.label(RichText::new(l).background_color(Color32::from_rgb(255, 241, 188)));
            }
            ui.add_space(16.);
            ui.label(RichText::new("Rappel actif").strong());
            for l in &f.recall {
                ui.label(format!("• {l}"));
            }
            ui.add_space(16.);
            ui.collapsing("Références vérifiables", |ui| {
                for c in &f.sources {
                    ui.label(format!("{} · p. {} : {}", c.document_id, c.page, c.quote));
                }
            });
        });
}

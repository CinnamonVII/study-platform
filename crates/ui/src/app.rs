use super::*;
impl eframe::App for StudyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_ready {
            theme(ctx);
            self.theme_ready = true;
        }
        self.process_replies();
        egui::TopBottomPanel::top("navigation")
            .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(16))
            .show(ctx, |ui| {
                if ctx.content_rect().width() < 640. {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Révisions").size(24.).strong().color(ACTION));
                        egui::ComboBox::from_id_salt("mobile-navigation")
                            .selected_text(match self.tab {
                                Tab::Today => "Aujourd’hui",
                                Tab::Documents => "Documents",
                                Tab::Fiches => "Fiches",
                                Tab::Practice => "Entraînement",
                                Tab::Sync => "Synchronisation",
                            })
                            .show_ui(ui, |ui| {
                                for (tab, label) in [
                                    (Tab::Today, "Aujourd’hui"),
                                    (Tab::Documents, "Documents"),
                                    (Tab::Fiches, "Fiches"),
                                    (Tab::Practice, "Entraînement"),
                                    (Tab::Sync, "Synchronisation"),
                                ] {
                                    ui.selectable_value(&mut self.tab, tab, label);
                                }
                            });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Révisions").size(24.).strong().color(ACTION));
                        ui.add_space(16.);
                        for (tab, label) in [
                            (Tab::Today, "Aujourd’hui"),
                            (Tab::Documents, "Documents"),
                            (Tab::Fiches, "Fiches"),
                            (Tab::Practice, "Entraînement"),
                            (Tab::Sync, "Synchronisation"),
                        ] {
                            if ui.selectable_label(self.tab == tab, label).clicked() {
                                self.tab = tab;
                            }
                        }
                    });
                }
            });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(if self.demo {
                        "Démonstration fictive"
                    } else {
                        "Espace privé connecté"
                    })
                    .strong()
                    .color(if self.demo { MUTED } else { GREEN }),
                );
                if self.busy {
                    ui.spinner();
                    ui.label("Traitement en cours…");
                }
                ui.label(&self.notice);
            });
        });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(244, 247, 250))
                    .inner_margin(if ctx.content_rect().width() < 600. {
                        16
                    } else {
                        32
                    }),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_max_width(ui.available_width().min(1040.));
                    ui.add_enabled_ui(!self.busy, |ui| match self.tab {
                        Tab::Today => self.today(ui),
                        Tab::Documents => self.documents(ui, ctx),
                        Tab::Fiches => self.fiches(ui, ctx),
                        Tab::Practice => self.practice(ui, ctx),
                        Tab::Sync => self.sync(ui, ctx),
                    });
                });
            });
        if let Some((kind, id, mut text)) = self.editor.take() {
            let mut open = true;
            let mut save = false;
            egui::Window::new("Corriger le contenu")
                .open(&mut open)
                .default_width(720.)
                .show(ctx, |ui| {
                    ui.label(
                        "Les corrections sont conservées et restent à valider avant utilisation.",
                    );
                    egui::ScrollArea::vertical()
                        .max_height(480.)
                        .show(ui, |ui| {
                            edit_form(ui, &kind, &mut text);
                        });
                    if !self.editor_error.is_empty() {
                        ui.colored_label(RED, &self.editor_error);
                    }
                    if primary(ui, "Enregistrer la correction") {
                        save = true;
                    }
                });
            if save {
                match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(value) => {
                        self.request(ctx, "PUT", &format!("/{kind}"), Some(value));
                        open = false;
                    }
                    Err(e) => self.editor_error = format!("Format JSON invalide : {e}"),
                }
            }
            if open {
                self.editor = Some((kind, id, text));
            }
        }
    }
}

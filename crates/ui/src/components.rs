use super::*;
pub(super) fn theme(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    let mut s = (*ctx.style()).clone();
    s.visuals.override_text_color = Some(INK);
    s.visuals.selection.bg_fill = Color32::from_rgb(214, 231, 244);
    s.visuals.selection.stroke = egui::Stroke::new(2_f32, ACTION);
    s.spacing.item_spacing = egui::vec2(12., 10.);
    s.spacing.button_padding = egui::vec2(14., 10.);
    s.spacing.interact_size = egui::vec2(44., 44.);
    s.text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.));
    s.text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(16.));
    s.text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(22.));
    s.wrap_mode = Some(egui::TextWrapMode::Wrap);
    s.animation_time = 0.;
    ctx.set_style(s);
}

pub(super) fn primary(ui: &mut egui::Ui, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE))
            .fill(ACTION)
            .min_size(egui::vec2(44., 44.)),
    )
    .clicked()
}

pub(super) fn title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(30.).strong().color(INK));
    ui.label(RichText::new(subtitle).color(MUTED));
    ui.add_space(24.);
}

pub(super) fn status(ui: &mut egui::Ui, s: Status) {
    let (text, c) = match s {
        Status::Pending => ("À valider", MUTED),
        Status::Accepted => ("Validé", GREEN),
        Status::Rejected => ("Rejeté", RED),
        Status::Stale => ("Source modifiée : à revérifier", RED),
    };
    ui.label(RichText::new(text).color(c).strong());
}

pub(super) fn citation_view(ui: &mut egui::Ui, c: &Citation, s: &Snapshot) {
    ui.label(format!("{} · p. {} : {}", c.document_id, c.page, c.quote));
    if let Some(d) = s.documents.iter().find(|d| d.id == c.document_id) {
        if d.original.starts_with("https://www.notion.so/") {
            ui.hyperlink_to("Consulter le document original", &d.original);
        } else {
            ui.label(&d.title);
        }
    }
}

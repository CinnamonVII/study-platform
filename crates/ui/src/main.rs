#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    eframe::run_native(
        "Révisions",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1160., 820.])
                .with_min_inner_size([320., 480.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(study_ui::StudyApp::new(cc)))),
    )
}
#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast;
    wasm_bindgen_futures::spawn_local(async {
        let doc = web_sys::window().unwrap().document().unwrap();
        let canvas = doc
            .get_element_by_id("study")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(study_ui::StudyApp::new(cc)))),
            )
            .await;
        if let Some(loading) = doc.get_element_by_id("loading") {
            match result{Ok(())=>loading.remove(),Err(_)=>loading.set_text_content(Some("Impossible de démarrer l’interface. Vérifiez que WebGL est disponible puis rechargez."))}
        }
    });
}

use crate::renderer::PostSettings;
use crate::scene::Scene;

pub fn panel(ctx: &egui::Context, scene: &mut Scene, post: &mut PostSettings, supersample: &mut bool) {
    egui::Window::new("Réglages")
        .default_open(true)
        .resizable(false)
        .show(ctx, |ui| {
            egui::CollapsingHeader::new("Disque d'accrétion")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut scene.disk.peak_temperature, 2000.0..=15000.0)
                            .text("Température pic (K)"),
                    );
                    ui.add(egui::Slider::new(&mut scene.disk.intensity, 0.0..=2.0).text("Intensité"));
                    ui.add(
                        egui::Slider::new(&mut scene.disk.inner_radius, 1.5..=6.0)
                            .text("Rayon interne"),
                    );
                    ui.add(
                        egui::Slider::new(&mut scene.disk.outer_radius, 6.0..=20.0)
                            .text("Rayon externe"),
                    );
                    if ui.button("Inverser la rotation").clicked() {
                        scene.disk.spin = -scene.disk.spin;
                    }
                });

            egui::CollapsingHeader::new("Trou noir").show(ui, |ui| {
                ui.add(
                    egui::Slider::new(&mut scene.black_hole.schwarzschild_radius, 0.5..=2.0)
                        .text("Rayon de Schwarzschild"),
                );
            });

            egui::CollapsingHeader::new("Rendu")
                .default_open(true)
                .show(ui, |ui| {
                    ui.add(egui::Slider::new(&mut post.exposure, 0.05..=2.0).text("Exposition"));
                    ui.add(egui::Slider::new(&mut post.bloom_strength, 0.0..=2.0).text("Bloom"));
                    ui.add(
                        egui::Slider::new(&mut post.bloom_threshold, 0.0..=4.0).text("Seuil bloom"),
                    );
                    ui.checkbox(supersample, "Anti-aliasing 2× (SSAA)");
                });
        });
}

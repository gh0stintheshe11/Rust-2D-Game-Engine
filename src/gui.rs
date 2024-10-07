use eframe::egui;

#[derive(Default)]
pub struct EngineGui;

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_three_panel_layout(ctx); // Render the layout
    }
}

impl EngineGui {
    fn show_three_panel_layout(&self, ctx: &egui::Context) {
        let window_width = ctx.screen_rect().width();
        let window_height = ctx.screen_rect().height();

        // Left panel (split into top and bottom)
        egui::SidePanel::left("asset")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(window_width * 0.15);

                // Split the left panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("asset_inspector")
                    .resizable(false)
                    .min_height(window_height * 0.5) // Top part of the left panel
                    .show_inside(ui, |ui| {
                        ui.heading("Asset Inspector");
                        ui.label("inspect and modfiy the attributes of the asset");
                    });

                egui::TopBottomPanel::bottom("asset_browser")
                    .resizable(false)
                    .min_height(window_height * 0.5) // Bottom part of the left panel
                    .show_inside(ui, |ui| {
                        ui.heading("Asset Browser");
                        ui.label("browse and select the asset");
                    });
            });

        // Right panel (15% of window width)
        egui::SidePanel::right("script")
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_width(window_width * 0.15);

                // Split the right panel into top and bottom using TopBottomPanel
                egui::TopBottomPanel::top("script_inspector")
                    .resizable(false)
                    .min_height(window_height * 0.5) // Top part of the left panel
                    .show_inside(ui, |ui| {
                        ui.heading("Script Inspector");
                        ui.label("inspect and modfiy the attributes of the script");
                    });

                egui::TopBottomPanel::bottom("script_browser")
                    .resizable(false)
                    .min_height(window_height * 0.5) // Bottom part of the left panel
                    .show_inside(ui, |ui| {
                        ui.heading("Script Browser");
                        ui.label("browse and select the script");
                    });
            });

        // Central panel (remaining space)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scene");
            ui.label("Scene Viewer");
        });

        // Bottom panel split into left and right
        egui::TopBottomPanel::bottom("terminal")
            .resizable(false)
            .min_height(window_height * 0.15) // Height of the bottom panel
            .show(ctx, |ui| {
                let bottom_panel_width = ui.available_width();
                ui.set_width(bottom_panel_width);
                ui.heading("Terminal");
                ui.label("Display the system output of the engine");
            });
    }
}
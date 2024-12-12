use eframe::egui;

pub struct EngineGui {
    // Window States
    show_hierarchy: bool,
    show_filesystem: bool,
    show_inspector: bool,
    show_console: bool,
    show_editor: bool,
    show_debug: bool,

    // Window Sizes (as percentages of screen size)
    side_panel_width_percentage: f32,
    console_height_percentage: f32,

    // GUI settings
    dark_mode: bool,
}

impl EngineGui {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            show_hierarchy: true,
            show_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_editor: false,
            show_debug: false,

            side_panel_width_percentage: 0.2, // 20% of screen width
            console_height_percentage: 0.2,    // 20% of screen height

            // GUI settings
            dark_mode: false,
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.available_rect();
        let side_panel_width = screen_rect.width() * self.side_panel_width_percentage;
        let console_height = screen_rect.height() * self.console_height_percentage;
        let menu_height = 32.0;
        let spacing = 4.0;

        // Compensate for window frame borders
        let border_compensation = 1.0; // Adjust this value as needed

        // Calculate available space after menu
        let available_height = screen_rect.height() - menu_height - console_height + border_compensation;
        let half_height = (available_height - spacing) / 2.0;

        // Frame color
        let default_fill = self.get_background_color();

        // Create menu frame with dark background
        let menu_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(4.0, 4.0),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        let menu_height = menu_frame.inner_margin.top 
            + menu_frame.inner_margin.bottom 
            + 20.0;

        // Create different frames for different window types
        let panel_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(4.0, 4.0),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        let viewport_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(4.0, 4.0),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        let console_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(4.0, 4.0),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        // Apply frames to windows...
        egui::Window::new("Menu")
            .frame(menu_frame)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(false)
            .fixed_size([screen_rect.width(), menu_height])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("File", |ui| {
                        ui.button("New Project");
                        ui.button("Open Project");
                        ui.button("Save Project");
                        ui.separator();
                        ui.button("Exit");
                    });
                    ui.menu_button("Edit", |ui| {
                        ui.button("Undo");
                        ui.button("Redo");
                    });
                    ui.menu_button("View", |ui| {
                        let dark_mode_button = ui.selectable_label(self.dark_mode, "Dark Mode");

                        // Handle dark mode button
                        if dark_mode_button.clicked() {
                            self.dark_mode = !self.dark_mode;
                            self.update_theme(ctx);
                        }
                    });
                    ui.menu_button("Import", |ui| {
                        ui.button("Import Sound");
                        ui.button("Import Image");
                        ui.button("Import Script");
                    });
                    ui.menu_button("Project", |ui| {
                        ui.button("Build Project");
                    });
                });
            });

        // Scene Hierarchy Window (Left Top)
        egui::Window::new("Scene Hierarchy")
            .frame(panel_frame)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, menu_height))
            .fixed_size([side_panel_width - border_compensation, half_height])
            .show(ctx, |ui| {
                ui.label("Scene tree will go here");
            });

        // File System Window (Left Bottom)
        egui::Window::new("File System")
            .frame(panel_frame)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0))
            .fixed_size([side_panel_width - border_compensation, half_height])
            .show(ctx, |ui| {
                ui.label("File browser will go here");
            });

        // Inspector Window (Right)
        egui::Window::new("Inspector")
            .frame(panel_frame)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(border_compensation, menu_height))
            .fixed_size([side_panel_width - border_compensation, available_height])
            .show(ctx, |ui| {
                ui.label("Properties will go here");
            });

        // Viewport (Center)
        egui::Window::new("Viewport")
            .frame(viewport_frame)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(
                side_panel_width + spacing - border_compensation,
                menu_height
            ))
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(false)
            .fixed_size([
                screen_rect.width() - (2.0 * side_panel_width) - (2.0 * spacing) + (2.0 * border_compensation),
                available_height
            ])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.show_editor, "🎮 Viewport").clicked() {
                        self.show_editor = false;
                    }
                    if ui.selectable_label(self.show_editor, "📝 Editor").clicked() {
                        self.show_editor = true;
                    }
                });
                ui.separator();

                if self.show_editor {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(40));
                    ui.add_sized(rect.size(),
                        egui::TextEdit::multiline(&mut String::new())
                            .code_editor()
                            .desired_width(f32::INFINITY)
                    );
                } else {
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().rect_filled(rect, 0.0, egui::Color32::from_gray(32));
                    ui.centered_and_justified(|ui| {
                        ui.label("Game view will go here");
                    });
                }
            });

        // Console/Debug Window (Bottom)
        egui::Window::new(if self.show_debug { "Debug" } else { "Console" })
            .frame(console_frame)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(
                side_panel_width + spacing - border_compensation,
                border_compensation
            ))
            .fixed_size([
                screen_rect.width() - (2.0 * side_panel_width) - (2.0 * spacing) + (2.0 * border_compensation),
                console_height - border_compensation
            ])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.show_debug, "🖥 Output").clicked() {
                        self.show_debug = false;
                    }
                    if ui.selectable_label(self.show_debug, "🔧 Debug").clicked() {
                        self.show_debug = true;
                    }
                });
                ui.separator();
                if self.show_debug {
                    ui.label("Debug info will go here");
                } else {
                    ui.label("Output console will go here");
                }
            });
    }

    fn get_background_color(&self) -> egui::Color32 {
        if self.dark_mode {
            egui::Color32::from_gray(30) // Dark gray
        } else {
            egui::Color32::from_gray(240) // Light gray
        }
    }

    fn update_theme(&mut self, ctx: &egui::Context) {
        ctx.set_visuals(if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        ctx.request_repaint();
    }
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, self.get_background_color());

            self.show_windows(ctx);
        });
    }
}

use eframe::egui;

pub struct EngineGui {
    // Layout States
    areas: Areas,
    
    // Panel States
    show_output_tab: bool,
    show_debug_tab: bool,
    show_editor: bool,
    show_debug: bool,
}

struct Areas {
    left_panel_width: f32,
    right_panel_width: f32,
    bottom_panel_height: f32,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            areas: Areas {
                left_panel_width: 250.0,
                right_panel_width: 250.0,
                bottom_panel_height: 200.0,
            },
            show_output_tab: true,
            show_debug_tab: false,
            show_editor: false,
            show_debug: false,
        }
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("top_bar").show_inside(ui, |ui| {
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
                    ui.button("Reset Layout");
                });

                ui.menu_button("Project", |ui| {
                    ui.button("Build");
                    ui.button("Run");
                });

                ui.menu_button("Help", |ui| {
                    ui.button("Documentation");
                    ui.button("About");
                });
            });
        });
    }

    fn show_left_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(self.areas.left_panel_width)
            .width_range(200.0..=400.0)
            .show_inside(ui, |ui| {
                // Calculate total available height and split point
                let available_height = ui.available_height();
                let split_height = available_height * 0.5;

                // Top section - Scene Hierarchy
                egui::TopBottomPanel::top("scene_hierarchy")
                    .exact_height(split_height)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.heading("Scene Hierarchy");
                        ui.separator();
                        ui.label("Scene tree will go here");
                });

                // Bottom section - File System
                egui::TopBottomPanel::bottom("file_system")
                    .exact_height(split_height)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.heading("File System");
                        ui.separator();
                        ui.label("File browser will go here");
                });
            });
    }

    fn show_right_panel(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(self.areas.right_panel_width)
            .width_range(200.0..=400.0)
            .show_inside(ui, |ui| {
                ui.heading("Inspector");
                ui.label("Properties will go here");
            });
    }

    fn show_bottom_panel(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(self.areas.bottom_panel_height)
            .height_range(100.0..=300.0)
            .show_inside(ui, |ui| {
                // Tab selection
                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.show_debug, "🖥 Output").clicked() {
                        self.show_debug = false;
                    }
                    if ui.selectable_label(self.show_debug, "🔧 Debug").clicked() {
                        self.show_debug = true;
                    }
                });
                ui.separator();

                // Content area
                if self.show_debug {
                    ui.label("Debug info will go here");
                } else {
                    ui.label("Output console will go here");
                }
            });
    }

    fn show_center_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Top toolbar for view switching
            ui.horizontal(|ui| {
                if ui.selectable_label(!self.show_editor, "🎮 Viewport").clicked() {
                    self.show_editor = false;
                }
                if ui.selectable_label(self.show_editor, "📝 Editor").clicked() {
                    self.show_editor = true;
                }
            });
            ui.separator();

            // Content area
            if self.show_editor {
                // Code Editor View
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_gray(40),
                );
                ui.add_sized(
                    rect.size(),
                    egui::TextEdit::multiline(&mut String::new())
                        .code_editor()
                        .desired_width(f32::INFINITY)
                );
            } else {
                // Viewport View
                let rect = ui.available_rect_before_wrap();
                ui.painter().rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_gray(32),
                );
                ui.centered_and_justified(|ui| {
                    ui.label("Game view will go here");
                });
            }
        });
    }
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_top_bar(ui);
            self.show_left_panel(ui);
            self.show_right_panel(ui);
            self.show_bottom_panel(ui);
            self.show_center_panel(ui);
        });
    }
}

use crate::gui::gui_state::GuiState;
use crate::gui::menu_bar::MenuBar;
use crate::gui::scene_hierarchy::SceneHierarchy;
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

    // Windows
    pub scene_hierarchy: SceneHierarchy,
    pub menu_bar: MenuBar,

    // GUI settings
    pub gui_state: GuiState,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Explicitly set dark mode visuals at startup
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let gui_state = GuiState::new();

        Self {
            show_hierarchy: true,
            show_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_editor: false,
            show_debug: false,
            side_panel_width_percentage: 0.2,
            console_height_percentage: 0.2,
            scene_hierarchy: SceneHierarchy::new(),
            menu_bar: MenuBar::new(),
            gui_state,
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.available_rect();
        let side_panel_width = screen_rect.width() * self.side_panel_width_percentage;
        let console_height = screen_rect.height() * self.console_height_percentage;
        let spacing = 4.0;

        // Frame color
        let default_fill = self.get_background_color();

        let viewport_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(spacing, spacing),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        let console_frame = egui::Frame {
            inner_margin: egui::Margin::symmetric(spacing, spacing),
            outer_margin: egui::Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: default_fill,
            stroke: ctx.style().visuals.widgets.noninteractive.bg_stroke,
        };

        // Viewport (Center)
        egui::Window::new("Main Window")
            .frame(viewport_frame)
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .title_bar(false)
            .fixed_size([
                screen_rect.width() - 2.0 * spacing,
                screen_rect.height() - 2.0 * spacing,
            ])
            .show(ctx, |ui| {
                // Menu bar at top
                ui.horizontal(|ui| {
                    self.menu_bar.show(ctx, ui, &mut self.gui_state);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(self.show_editor, "📝 Editor").clicked() {
                            self.show_editor = true;
                        }
                        if ui.selectable_label(!self.show_editor, "🎮 Viewport").clicked() {
                            self.show_editor = false;
                        }
                    });
                });
                ui.separator();

                // Main content area with resizable panels
                let available_rect = ui.available_rect_before_wrap();
                
                // Left panel (Scene/Files)
                egui::SidePanel::left("left_panel")
                    .resizable(true)
                    .min_width(200.0)
                    .max_width(available_rect.width() * 0.4)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::ZERO))
                    .show_inside(ui, |ui| {
                        // Use vertical layout to split the panel
                        egui::TopBottomPanel::top("scene_panel")
                            .resizable(true)
                            .min_height(200.0)
                            .default_height(ui.available_height() * 0.5)
                            .show_inside(ui, |ui| {
                                ui.heading("Scene");
                                ui.separator();
                                self.scene_hierarchy.show(ui);
                            });

                        egui::CentralPanel::default().show_inside(ui, |ui| {
                            ui.heading("Files");
                            ui.separator();
                            ui.label("File browser will go here");
                        });
                    });

                // Right panel (Inspector)
                egui::SidePanel::right("right_panel")
                    .resizable(true)
                    .min_width(200.0)
                    .max_width(available_rect.width() * 0.4)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 0.0)))
                    .show_inside(ui, |ui| {
                        ui.heading("Inspector");
                        ui.separator();
                        ui.label("Properties will go here");
                    });

                // Center panel (Game view/Editor)
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let content_rect = ui.available_rect_before_wrap();
                    if self.show_editor {
                        ui.painter().rect_filled(content_rect, 0.0, egui::Color32::from_gray(40));
                        ui.add_sized(
                            content_rect.size(),
                            egui::TextEdit::multiline(&mut String::new())
                                .code_editor()
                                .desired_width(f32::INFINITY),
                        );
                    } else {
                        ui.painter().rect_filled(content_rect, 0.0, egui::Color32::from_gray(32));
                        ui.centered_and_justified(|ui| {
                            ui.label("Game view will go here");
                        });
                    }
                });
            });

        // Console/Debug Window (Bottom)
        egui::Window::new(if self.show_debug { "Debug" } else { "Console" })
            .frame(console_frame)
            .order(egui::Order::Foreground)
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -spacing),
            )
            .fixed_size([
                screen_rect.width() - (2.0 * side_panel_width) - (2.0 * spacing),
                console_height - spacing,
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
        if self.gui_state.dark_mode {
            egui::Color32::from_gray(30) // Dark gray
        } else {
            egui::Color32::from_gray(240) // Light gray
        }
    }
}

impl eframe::App for EngineGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, 0.0, self.get_background_color());

            self.show_windows(ctx);
        });
    }
}

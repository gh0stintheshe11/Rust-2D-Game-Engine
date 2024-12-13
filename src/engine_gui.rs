use crate::ecs::{AttributeValue, Entity, Resource, ResourceType, Scene};
use crate::gui::gui_state::GuiState;
use crate::gui::menu_bar::MenuBar;
use crate::gui::scene_hierarchy::SceneHierarchy;
use crate::input_handler::{InputContext, InputHandler};
use crate::render_engine::RenderEngine;
use eframe::egui;
use uuid::Uuid;

pub struct EngineGui {
    // Window States
    show_hierarchy: bool,
    show_filesystem: bool,
    show_inspector: bool,
    show_console: bool,
    show_editor: bool,
    show_debug: bool,

    // Windows
    pub scene_hierarchy: SceneHierarchy,
    pub menu_bar: MenuBar,

    // GUI settings
    pub gui_state: GuiState,

    // Add render engine
    render_engine: RenderEngine,

    // Add input handler
    input_handler: InputHandler,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let gui_state = GuiState::new();

        let mut render_engine = RenderEngine::new();

        Self {
            show_hierarchy: true,
            show_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_editor: false,
            show_debug: false,
            scene_hierarchy: SceneHierarchy::new(),
            menu_bar: MenuBar::new(),
            gui_state,
            render_engine,
            input_handler: InputHandler::new(),
        }
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.available_rect();
        let spacing = 4.0;
        let min_side_panel_width = 200.0;
        let min_console_height = 200.0;

        // Frame color
        let default_fill = self.get_background_color();

        let main_window_frame = egui::Frame {
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
            .frame(main_window_frame)
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
                        if ui
                            .selectable_label(!self.show_editor, "🎮 Viewer")
                            .clicked()
                        {
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
                    .min_width(min_side_panel_width)
                    .max_width(available_rect.width() * 0.4)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::ZERO))
                    .show_inside(ui, |ui| {
                        // Use vertical layout to split the panel
                        egui::TopBottomPanel::top("scene_panel")
                            .resizable(true)
                            .min_height(200.0)
                            .max_height(ui.available_height() * 0.75)
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
                    .min_width(min_side_panel_width)
                    .max_width(available_rect.width() * 0.4)
                    .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 0.0)))
                    .show_inside(ui, |ui| {
                        ui.heading("Inspector");
                        ui.separator();
                        ui.label("Properties will go here");
                    });

                // Center panel (Game view/Editor)
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(2.0, 2.0)))
                    .show_inside(ui, |ui| {
                        let content_rect = ui.available_rect_before_wrap();

                        // First fill the background
                        if self.show_editor {
                            ui.painter().rect_filled(
                                content_rect,
                                0.0,
                                egui::Color32::from_gray(40),
                            );
                        } else {
                            ui.painter().rect_filled(
                                content_rect,
                                0.0,
                                egui::Color32::from_gray(32),
                            );
                        }

                        // Main content
                        if self.show_editor {
                            ui.add_sized(
                                content_rect.size(),
                                egui::TextEdit::multiline(&mut String::new())
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        } else {
                            // Render the game view first
                            self.render_test_scene(ui);

                            // Get viewport rect for input handling
                            let viewport_rect = ui.max_rect();

                            // Game control buttons floating on top
                            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.add_space((ui.available_width() - 170.0) * 0.5);
                                    if ui.button("▶ Play").clicked() {
                                        // Handle play
                                    }
                                    if ui.button("⏸ Pause").clicked() {
                                        // Handle pause
                                    }
                                    if ui.button("⏹ Reset").clicked() {
                                        // Handle game reset
                                    }
                                });
                            });

                            // Camera reset button in bottom right
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                                ui.add_space(4.0);  // Bottom margin
                                ui.horizontal(|ui| {
                                    ui.add_space(4.0);  // Right margin
                                    if ui.button("⭕").clicked() {
                                        self.render_engine.camera.reset();
                                    }
                                });
                            });

                            // Update input handler
                            ui.ctx().input(|input| {
                                self.input_handler.handle_input(input);
                            });

                            // Only handle game input if cursor is in viewport
                            if let Some(cursor_pos) = ui.ctx().input(|i| i.pointer.hover_pos()) {
                                if viewport_rect.contains(cursor_pos) {
                                    // Handle camera pan with mouse drag
                                    if self.input_handler.is_mouse_button_pressed(egui::PointerButton::Secondary) || 
                                       (self.input_handler.is_mouse_button_pressed(egui::PointerButton::Primary) && 
                                        ui.ctx().input(|i| i.modifiers.alt)) {
                                        ui.ctx().input(|i| {
                                            let delta = i.pointer.delta();
                                            self.render_engine.camera.move_by(-delta.x, -delta.y);
                                        });
                                    }

                                    // Handle zoom with mouse wheel
                                    if let Some(scroll_delta) = self.input_handler.get_scroll_delta() {
                                        let zoom_factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                                        self.render_engine.camera.zoom_by(zoom_factor);
                                    }
                                }
                            }
                        }
                    });
            });

        // Console/Debug Window (Bottom)
        egui::Window::new(if self.show_debug { "Debug" } else { "Console" })
            .frame(console_frame)
            .order(egui::Order::Foreground)
            .resizable(true)
            .collapsible(true)
            .movable(false)
            .title_bar(true)
            .vscroll(true)
            .max_width(screen_rect.width() - 2.0 * min_side_panel_width - 4.0 * spacing)
            .max_height(screen_rect.height() * 0.5)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, 0.0))
            .default_size([
                screen_rect.width() - (2.0 * min_side_panel_width) - (4.0 * spacing),
                min_console_height,
            ])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.show_debug, "💬 Output").clicked() {
                        self.show_debug = false;
                    }
                    if ui.selectable_label(self.show_debug, "🛠 Debug").clicked() {
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

    fn render_test_scene(&mut self, ui: &mut egui::Ui) {
        let (width, height) = (ui.available_width(), ui.available_height());
        println!("Viewport size: {}x{}", width, height);

        // Get camera-transformed positions
        let center_screen = self.render_engine.camera.world_to_screen((0.0, 0.0));
        let square_screen = self.render_engine.camera.world_to_screen((-100.0, -100.0));

        // Draw a red circle
        ui.painter().circle_filled(
            egui::pos2(
                width / 2.0 + center_screen.0,
                height / 2.0 + center_screen.1,
            ), // Center position
            30.0 * self.render_engine.camera.zoom, // Scale radius with zoom
            egui::Color32::RED,
        );

        // Draw a blue square
        let square_size = 50.0 * self.render_engine.camera.zoom; // Scale size with zoom
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(
                    width / 2.0 + square_screen.0,
                    height / 2.0 + square_screen.1,
                ),
                egui::vec2(square_size, square_size),
            ),
            0.0, // rounding
            egui::Color32::BLUE,
        );

        // Debug print camera state
        println!(
            "Camera pos: {:?}, zoom: {}",
            self.render_engine.camera.position, self.render_engine.camera.zoom
        );
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

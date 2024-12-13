use crate::gui::gui_state::GuiState;
use crate::gui::menu_bar::MenuBar;
use crate::gui::scene_hierarchy::SceneHierarchy;
use crate::render_engine::RenderEngine;
use crate::ecs::{Scene, Entity, Resource, ResourceType, AttributeValue};
use eframe::egui;
use uuid::Uuid;
use crate::input_handler::{InputHandler, InputContext};

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

    // Add render engine
    render_engine: RenderEngine,
    test_scene: Scene,  // For testing

    // Add input handler
    input_handler: InputHandler,
}

impl EngineGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let gui_state = GuiState::new();

        // Create test scene
        let mut test_scene = Scene::new("TestScene");
        let mut render_engine = RenderEngine::new();
        
        // Create a test entity with proper ECS
        let entity_id = test_scene.create_entity("TestSquare");
        if let Some(test_entity) = test_scene.get_entity_mut(entity_id) {
            test_entity.create_attribute(
                "position",
                crate::ecs::AttributeType::Vector2,
                AttributeValue::Vector2(0.0, 0.0)
            );
            test_entity.create_attribute(
                "rotation",
                crate::ecs::AttributeType::Float,
                AttributeValue::Float(0.0)
            );
            test_entity.create_attribute(
                "scale",
                crate::ecs::AttributeType::Vector2,
                AttributeValue::Vector2(1.0, 1.0)
            );
            test_entity.create_attribute(
                "layer",
                crate::ecs::AttributeType::Integer,
                AttributeValue::Integer(1)
            );
        }
        
        // Create and load a test texture as a resource
        let resource_id = test_scene.create_resource(
            "TestTexture",
            "assets/test_texture.png",
            ResourceType::Image
        );
        
        // Load the texture into the render engine
        if let Some(resource) = test_scene.get_resource(resource_id) {
            println!("Found resource: {}", resource.file_path);
            if let Ok(texture_id) = render_engine.load_texture(resource) {
                println!("Successfully loaded texture with ID: {}", texture_id);
                // Add the sprite attribute to the entity
                if let Some(test_entity) = test_scene.get_entity_mut(entity_id) {
                    test_entity.create_attribute(
                        "sprite",
                        crate::ecs::AttributeType::String,
                        AttributeValue::String(texture_id.to_string())
                    );
                    println!("Added sprite attribute to entity");
                }
            } else {
                println!("Failed to load texture!");
            }
        } else {
            println!("Could not find resource!");
        }

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
            render_engine,
            test_scene,
            input_handler: InputHandler::new(),
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
                        if ui
                            .selectable_label(!self.show_editor, "🎮 Viewport")
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
                            // Game control buttons floating on top (only in viewport mode)
                            ui.with_layout(
                                egui::Layout::top_down_justified(egui::Align::Center),
                                |ui| {
                                    ui.add_space(8.0); // Add top margin
                                    ui.horizontal(|ui| {
                                        ui.add_space((ui.available_width() - 170.0) * 0.5); // (half of the viewport width - buotton group width)/2
                                        if ui.button("▶ Play").clicked() {
                                            // Handle play
                                        }
                                        if ui.button("⏸ Pause").clicked() {
                                            // Handle pause
                                        }
                                        if ui.button("⟲ Reset").clicked() {
                                            // Handle reset
                                        }
                                    });
                                },
                            );

                            // Update input handler with more detailed input state
                            ui.ctx().input(|input| {
                                self.input_handler.handle_input(input);
                            });

                            // Debug print mouse state
                            println!("Mouse buttons: Middle={}, Primary={}, Alt={}", 
                                self.input_handler.is_mouse_button_pressed(egui::PointerButton::Middle),
                                self.input_handler.is_mouse_button_pressed(egui::PointerButton::Primary),
                                ui.ctx().input(|i| i.modifiers.alt)
                            );

                            // Handle camera pan with mouse drag
                            if self.input_handler.is_mouse_button_pressed(egui::PointerButton::Middle) || 
                               (self.input_handler.is_mouse_button_pressed(egui::PointerButton::Primary) && 
                                ui.ctx().input(|i| i.modifiers.alt)) {
                                ui.ctx().input(|i| {
                                    let delta = i.pointer.delta();  // delta is Vec2, not Option<Vec2>
                                    println!("Mouse delta from egui: {:?}", delta);
                                    self.render_engine.camera.move_by(-delta.x, -delta.y);
                                });
                            }

                            // Handle zoom with mouse wheel
                            if let Some(scroll_delta) = self.input_handler.get_scroll_delta() {
                                println!("Scroll delta: {:?}", scroll_delta);
                                let zoom_factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
                                self.render_engine.camera.zoom_by(zoom_factor);
                            }

                            // Print camera state
                            println!("Camera position: {:?}, zoom: {}", 
                                self.render_engine.camera.position, 
                                self.render_engine.camera.zoom);

                            // Render the game view
                            self.render_test_scene(ui);
                        }
                    });
            });

        // Console/Debug Window (Bottom)
        egui::Window::new(if self.show_debug { "Debug" } else { "Console" })
            .frame(console_frame)
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -spacing))
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

    fn render_test_scene(&mut self, ui: &mut egui::Ui) {
        let (width, height) = (ui.available_width(), ui.available_height());
        println!("Viewport size: {}x{}", width, height);
        
        // Get camera-transformed positions
        let center_screen = self.render_engine.camera.world_to_screen((0.0, 0.0));
        let square_screen = self.render_engine.camera.world_to_screen((-100.0, -100.0));
        
        // Draw a red circle
        ui.painter().circle_filled(
            egui::pos2(
                width/2.0 + center_screen.0,
                height/2.0 + center_screen.1
            ),  // Center position
            30.0 * self.render_engine.camera.zoom,  // Scale radius with zoom
            egui::Color32::RED,
        );
        
        // Draw a blue square
        let square_size = 50.0 * self.render_engine.camera.zoom;  // Scale size with zoom
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(
                    width/2.0 + square_screen.0,
                    height/2.0 + square_screen.1
                ),
                egui::vec2(square_size, square_size),
            ),
            0.0,  // rounding
            egui::Color32::BLUE,
        );

        // Debug print camera state
        println!("Camera pos: {:?}, zoom: {}", 
            self.render_engine.camera.position,
            self.render_engine.camera.zoom);
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

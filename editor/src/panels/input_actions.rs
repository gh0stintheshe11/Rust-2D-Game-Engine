//! The Input Actions window (Project → Input Actions…): the project's named
//! controls, which scripts read with `is_action_pressed("jump")` and
//! friends, and what each is bound to. Changes are saved to `project.epm`
//! right away and apply the next time the game plays.

use crate::state::GuiState;
use eframe::egui;
use rust_2d_game_engine::input::{
    AxisDirection, GAMEPAD_BUTTONS, GamepadAxis, InputActions, InputBinding,
};

/// The stick and trigger directions offered, in menu order.
const AXIS_BINDINGS: [(GamepadAxis, AxisDirection); 10] = [
    (GamepadAxis::LeftStickX, AxisDirection::Negative),
    (GamepadAxis::LeftStickX, AxisDirection::Positive),
    (GamepadAxis::LeftStickY, AxisDirection::Negative),
    (GamepadAxis::LeftStickY, AxisDirection::Positive),
    (GamepadAxis::RightStickX, AxisDirection::Negative),
    (GamepadAxis::RightStickX, AxisDirection::Positive),
    (GamepadAxis::RightStickY, AxisDirection::Negative),
    (GamepadAxis::RightStickY, AxisDirection::Positive),
    (GamepadAxis::LeftTrigger, AxisDirection::Positive),
    (GamepadAxis::RightTrigger, AxisDirection::Positive),
];

const MOUSE_BUTTONS: [egui::PointerButton; 5] = [
    egui::PointerButton::Primary,
    egui::PointerButton::Secondary,
    egui::PointerButton::Middle,
    egui::PointerButton::Extra1,
    egui::PointerButton::Extra2,
];

/// Gamepads the window offers new gamepad bindings for: any, or one of the
/// first few (for local multiplayer).
const GAMEPAD_CHOICES: usize = 4;

/// A change to the project's actions, asked for while drawing the window
/// and made after it.
enum Edit {
    Add(String),
    Remove(String),
    Rename(String, String),
    AddBinding(String, InputBinding),
    RemoveBinding(String, usize),
    /// A new threshold, and whether to save it (not while it's dragged)
    SetThreshold(String, f32, bool),
}

#[derive(Default)]
pub struct InputActionsWindow {
    pub open: bool,
    /// The action waiting for a key to be pressed
    capturing: Option<String>,
    /// The action being renamed, and the name typed so far
    renaming: Option<(String, String)>,
    /// The name of an action to add, as typed so far
    new_name: String,
    /// The gamepad new gamepad bindings are for (`None`: any)
    gamepad: Option<usize>,
    /// Why the last change was refused
    error: Option<String>,
    /// An action just added or renamed, to scroll to
    scroll_to: Option<String>,
    /// A threshold is being dragged, and its change isn't saved yet
    unsaved: bool,
}

impl InputActionsWindow {
    /// While waiting for a key, bind the first key pressed this frame and
    /// take it out of the frame's input, so the editor's shortcuts don't
    /// act on it too (Delete would delete the selected entity). Call it
    /// before they run.
    pub fn capture_key(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        let Some(action) = self.capturing.clone() else {
            return;
        };
        if !self.open {
            self.capturing = None;
            return;
        }
        let key = ctx.input_mut(|input| {
            let index = input.events.iter().position(|event| {
                matches!(
                    event,
                    egui::Event::Key {
                        pressed: true,
                        repeat: false,
                        ..
                    }
                )
            })?;
            match input.events.remove(index) {
                egui::Event::Key { key, .. } => Some(key),
                _ => None,
            }
        });
        if let Some(key) = key {
            self.capturing = None;
            self.apply(gui_state, Edit::AddBinding(action, InputBinding::Key(key)));
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, gui_state: &mut GuiState) {
        if !self.open {
            return;
        }
        let Some(actions) = gui_state
            .project_metadata
            .as_ref()
            .map(|metadata| metadata.input_actions.clone())
        else {
            self.open = false;
            return;
        };
        let mut open = true;
        let mut edit = None;
        // A size of its own, with the actions scrolling below the rest
        // (which stays in view): a window sized to its contents would move
        // as they grow
        egui::Window::new("Input Actions")
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .default_size([440.0, 480.0])
            .show(ctx, |ui| {
                ui.label(
                    "Named controls, which scripts read with is_action_pressed(\"jump\") and \
                     friends. Changes apply the next time the game plays.",
                );
                ui.horizontal(|ui| {
                    ui.label("New gamepad bindings are for");
                    let label = |pad: Option<usize>| match pad {
                        None => "any gamepad".to_string(),
                        Some(index) => format!("gamepad {}", index + 1),
                    };
                    egui::ComboBox::from_id_salt("input_actions_gamepad")
                        .selected_text(label(self.gamepad))
                        .show_ui(ui, |ui| {
                            for pad in std::iter::once(None).chain((0..GAMEPAD_CHOICES).map(Some)) {
                                ui.selectable_value(&mut self.gamepad, pad, label(pad));
                            }
                        });
                });
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.new_name)
                            .hint_text("new action, e.g. jump")
                            .desired_width(180.0),
                    );
                    let entered =
                        field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button("Add action").clicked() || entered {
                        edit = Some(Edit::Add(self.new_name.trim().to_string()));
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(egui::Color32::RED, error);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (name, bindings) in actions.iter() {
                            ui.separator();
                            let threshold = actions.threshold(name).unwrap_or_default();
                            self.action(ui, name, bindings, threshold, &mut edit);
                        }
                    });
            });
        if !open {
            *self = Self {
                gamepad: self.gamepad,
                ..Self::default()
            };
            return;
        }
        if let Some(edit) = edit {
            self.apply(gui_state, edit);
        }
    }

    /// One action: its name, then its bindings and a menu to add more, and
    /// with a stick or trigger among them, its threshold.
    fn action(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        bindings: &[InputBinding],
        threshold: f32,
        edit: &mut Option<Edit>,
    ) {
        if self.scroll_to.as_deref() == Some(name) {
            // At once: the row is there to be used
            ui.scroll_to_cursor_animation(
                Some(egui::Align::Center),
                egui::style::ScrollAnimation::none(),
            );
            self.scroll_to = None;
        }
        ui.horizontal(|ui| {
            match &mut self.renaming {
                Some((original, text)) if original == name => {
                    let field = ui.add(egui::TextEdit::singleline(text).desired_width(180.0));
                    if field.lost_focus() {
                        if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            *edit = Some(Edit::Rename(original.clone(), text.trim().to_string()));
                        }
                        self.renaming = None;
                    } else if !field.has_focus() {
                        // Just opened: type right away
                        field.request_focus();
                    }
                }
                _ => {
                    ui.strong(name);
                    if ui.small_button("✏").on_hover_text("Rename").clicked() {
                        self.renaming = Some((name.to_string(), name.to_string()));
                    }
                }
            }
            if ui
                .small_button("🗑")
                .on_hover_text("Remove the action")
                .clicked()
            {
                *edit = Some(Edit::Remove(name.to_string()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (index, binding) in bindings.iter().enumerate() {
                let chip = ui
                    .button(format!("{}  ✖", binding.label()))
                    .on_hover_text(format!("{} (click to remove)", binding));
                if chip.clicked() {
                    *edit = Some(Edit::RemoveBinding(name.to_string(), index));
                }
            }
            if self.capturing.as_deref() == Some(name) {
                ui.label("Press a key…");
                if ui.small_button("Cancel").clicked() {
                    self.capturing = None;
                }
                return;
            }
            ui.menu_button("+ Add", |ui| {
                let mut add = |ui: &mut egui::Ui, binding: InputBinding| {
                    if ui.button(binding.label()).clicked() {
                        *edit = Some(Edit::AddBinding(name.to_string(), binding));
                        ui.close();
                    }
                };
                if ui.button("Press a key…").clicked() {
                    self.capturing = Some(name.to_string());
                    ui.close();
                }
                ui.menu_button("Mouse button", |ui| {
                    for button in MOUSE_BUTTONS {
                        add(ui, InputBinding::Mouse(button));
                    }
                });
                let pad = self.gamepad;
                ui.menu_button("Gamepad button", |ui| {
                    for (button, _) in GAMEPAD_BUTTONS {
                        add(ui, InputBinding::GamepadButton { button, pad });
                    }
                });
                ui.menu_button("Gamepad stick or trigger", |ui| {
                    for (axis, direction) in AXIS_BINDINGS {
                        add(
                            ui,
                            InputBinding::GamepadAxis {
                                axis,
                                direction,
                                pad,
                            },
                        );
                    }
                });
            });
        });
        let analog = bindings
            .iter()
            .any(|binding| matches!(binding, InputBinding::GamepadAxis { .. }));
        if analog {
            ui.horizontal(|ui| {
                ui.label("A stick or trigger presses it from");
                let mut value = threshold;
                let drag = ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(0.05..=1.0)
                            .speed(0.01)
                            .fixed_decimals(2),
                    )
                    .on_hover_text(
                        "How far it's pushed, from 0 to 1. Higher keeps a stick pushed \
                         diagonally from pressing it.",
                    );
                // Saved once a drag ends, not on every frame of it
                let done = drag.drag_stopped() || (drag.changed() && !drag.dragged());
                if drag.changed() || done {
                    *edit = Some(Edit::SetThreshold(name.to_string(), value, done));
                }
            });
        }
    }

    /// Make a change to the project's actions and save the project, or say
    /// why it can't be made.
    fn apply(&mut self, gui_state: &mut GuiState, edit: Edit) {
        let Some(metadata) = gui_state.project_metadata.as_mut() else {
            return;
        };
        let mut actions = metadata.input_actions.clone();
        let mut save = true;
        let result = match edit {
            Edit::Add(name) => {
                if actions.bindings(&name).is_some() {
                    Err(format!("there's already an action named \"{}\"", name))
                } else {
                    actions.set(&name, Vec::new()).map(|()| {
                        self.new_name.clear();
                        self.scroll_to = Some(name);
                    })
                }
            }
            Edit::Remove(name) => {
                actions.remove(&name);
                Ok(())
            }
            Edit::Rename(name, new_name) => actions
                .rename(&name, &new_name)
                .map(|()| self.scroll_to = Some(new_name)),
            Edit::AddBinding(name, binding) => add_binding(&mut actions, &name, binding),
            Edit::RemoveBinding(name, index) => {
                let mut bindings = actions.bindings(&name).unwrap_or_default().to_vec();
                if index < bindings.len() {
                    bindings.remove(index);
                }
                actions.set(&name, bindings)
            }
            Edit::SetThreshold(name, threshold, done) => {
                save = done;
                actions.set_threshold(&name, threshold)
            }
        };
        match result {
            Ok(()) => {
                self.error = None;
                let changed = actions != metadata.input_actions;
                metadata.input_actions = actions;
                if save && (changed || self.unsaved) {
                    self.unsaved = false;
                    let _ = gui_state.save_project();
                } else if changed {
                    // Dragged: saved when the drag ends
                    self.unsaved = true;
                }
            }
            Err(e) => self.error = Some(e),
        }
    }
}

/// Add a binding to an action, unless it has it already.
fn add_binding(
    actions: &mut InputActions,
    name: &str,
    binding: InputBinding,
) -> Result<(), String> {
    let mut bindings = actions.bindings(name).unwrap_or_default().to_vec();
    if !bindings.contains(&binding) {
        bindings.push(binding);
    }
    actions.set(name, bindings)
}

use crate::ecs::SceneManager;
use crate::project_manager::ProjectMetadata;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub enum SelectedItem {
    None,
    Scene(Uuid),
    Entity(Uuid, Uuid),         // (Scene ID, Entity ID)
    Asset(Uuid, Uuid, PathBuf), // (Scene ID, Entity ID, Asset Path)
    File(PathBuf),
}

pub enum ScenePanelSelectedItem {
    None,
    Scene(Uuid),
    Entity(Uuid, Uuid),
    Asset(Uuid, Uuid, PathBuf), // (Scene ID, Entity ID, Asset Path)
}

/// Application exit flow, driven by File > Exit or the window close button.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExitRequest {
    None,
    /// The confirmation dialog is open
    PromptOpen,
    /// Save the project (and editor buffer), then close
    SaveAndExit,
    /// Close without saving
    ExitWithoutSaving,
}

/// Snapshot-based undo/redo over the scene manager.
///
/// Every completed editor mutation *commits* the resulting state (the same
/// places that save the project). Undo restores the previous committed
/// state; redo walks forward again. `states` always ends with the current
/// committed state.
pub struct UndoStack {
    states: Vec<SceneManager>,
    redo: Vec<SceneManager>,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoStack {
    const LIMIT: usize = 50;

    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Start a fresh history (project open / new project).
    pub fn reset(&mut self, initial: &SceneManager) {
        self.states = vec![initial.clone()];
        self.redo.clear();
    }

    /// Record the state after a completed mutation.
    pub fn commit(&mut self, state: &SceneManager) {
        self.states.push(state.clone());
        if self.states.len() > Self::LIMIT {
            self.states.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        self.states.len() >= 2
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Step back one committed state; returns the state to restore.
    pub fn undo(&mut self) -> Option<SceneManager> {
        if self.states.len() < 2 {
            return None;
        }
        let current = self.states.pop().expect("len checked");
        self.redo.push(current);
        Some(self.states.last().expect("len checked").clone())
    }

    /// Step forward one undone state; returns the state to restore.
    pub fn redo(&mut self) -> Option<SceneManager> {
        let state = self.redo.pop()?;
        self.states.push(state.clone());
        if self.states.len() > Self::LIMIT {
            self.states.remove(0);
        }
        Some(state)
    }
}

pub struct GuiState {
    pub dark_mode: bool,
    pub show_new_project_popup: bool,
    pub show_open_project_popup: bool,
    pub load_project: bool,    // Track if the project should be loaded
    pub project_name: String,  // Store the project name input
    pub project_path: PathBuf, // Store the project path input
    pub project_metadata: Option<ProjectMetadata>, // Store loaded project metadata
    pub scene_manager: Option<SceneManager>,

    pub show_hierarchy_filesystem: bool,
    pub show_inspector: bool,
    pub show_console: bool,
    pub show_debug_overlay: bool,

    pub selected_item: SelectedItem,
    pub scene_panel_selected_item: ScenePanelSelectedItem,

    pub build_result: Arc<Mutex<Option<Result<(), String>>>>,
    pub is_building: Arc<Mutex<bool>>,
    pub show_build_project_popup: bool,

    pub exit_request: ExitRequest,

    /// Set by any panel that wants a script opened in the code editor;
    /// consumed by the editor shell each frame.
    pub open_script_request: Option<PathBuf>,

    /// Snippet to insert at the script editor's cursor (e.g. clicking an
    /// attribute in the inspector); consumed by the editor shell each frame.
    pub script_insert_request: Option<String>,

    pub undo_stack: UndoStack,
}

impl Default for GuiState {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            dark_mode: true,
            show_new_project_popup: false,
            show_open_project_popup: false,
            load_project: false,
            project_name: String::new(),
            project_path: PathBuf::new(),
            project_metadata: None,
            scene_manager: None,

            show_hierarchy_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_debug_overlay: false,

            selected_item: SelectedItem::None,
            scene_panel_selected_item: ScenePanelSelectedItem::None,

            build_result: Arc::new(Mutex::new(None)),
            is_building: Arc::new(Mutex::new(false)),
            show_build_project_popup: false,

            exit_request: ExitRequest::None,

            open_script_request: None,

            script_insert_request: None,

            undo_stack: UndoStack::new(),
        }
    }
}

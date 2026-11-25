//! Editor state shared by all panels, and the operations that keep it
//! consistent (opening projects, saving, undo/redo, selection).

use rust_2d_game_engine::logger::LOGGER;
use rust_2d_game_engine::project::{LoadedProject, ProjectManager, ProjectMetadata};
use rust_2d_game_engine::scene::{AttributeValue, Entity, SceneManager};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// How far a duplicated or pasted entity moves from an entity of the same
/// name it would otherwise exactly cover, in world units along x and y.
pub const COPY_OFFSET: f32 = 16.0;

/// What painting a tilemap puts in a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileBrush {
    /// This tile of the tileset
    Tile(u32),
    /// Nothing: empties the cell
    Eraser,
}

/// What the inspector shows.
pub enum SelectedItem {
    None,
    Scene(Uuid),
    Entity(Uuid, Uuid), // (Scene ID, Entity ID)
    File(PathBuf),
}

/// What is highlighted in the scene hierarchy (files don't count).
pub enum ScenePanelSelectedItem {
    None,
    Scene(Uuid),
    Entity(Uuid, Uuid),
}

/// Application exit flow, driven by File > Exit or the window close button.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExitRequest {
    None,
    /// The confirmation dialog is open
    PromptOpen,
    /// Save the script editor buffer, then close
    SaveAndExit,
    /// Close, discarding unsaved script edits
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

    /// Record the state after a completed mutation. A state equal to the
    /// current one is not recorded (and doesn't clear the redo history).
    pub fn commit(&mut self, state: &SceneManager) {
        if self.states.last() == Some(state) {
            return;
        }
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
    /// Whether a project is open
    pub project_open: bool,
    pub project_name: String,
    pub project_path: PathBuf,
    pub project_metadata: Option<ProjectMetadata>,
    pub scene_manager: Option<SceneManager>,
    /// Incremented whenever a project is opened or created, so the app
    /// shell can reset everything tied to the previous project.
    pub project_generation: u64,

    pub show_hierarchy_filesystem: bool,
    pub show_inspector: bool,
    pub show_console: bool,
    pub show_debug_overlay: bool,
    /// Draw physics collider outlines while the game plays
    pub show_colliders: bool,
    /// Round dragged entities' positions to the grid (Shift inverts it)
    pub snap_to_grid: bool,
    /// The viewport grid's cell size, in world units
    pub grid_size: f32,
    /// Left-dragging in the viewport paints the selected tilemap
    pub painting_tiles: bool,
    pub tile_brush: TileBrush,

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

    /// File > Save Project and Project > Build: handled by the shell, which
    /// saves the script editor buffer first.
    pub save_requested: bool,
    pub build_requested: bool,
    /// Project > Clear Save Data: handled by the shell, which owns the game
    pub clear_save_data_requested: bool,

    /// A file the user deleted from the file panel this frame.
    pub deleted_file: Option<PathBuf>,

    pub undo_stack: UndoStack,

    /// Clipboard text of the last entity copied, for the hierarchy's Paste
    /// menu item (Ctrl+V pastes from the system clipboard). Text, not an
    /// entity, so pasting into another project resolves its paths there.
    pub copied_entity: Option<String>,
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
            project_open: false,
            project_name: String::new(),
            project_path: PathBuf::new(),
            project_metadata: None,
            scene_manager: None,
            project_generation: 0,

            show_hierarchy_filesystem: true,
            show_inspector: true,
            show_console: true,
            show_debug_overlay: false,
            show_colliders: true,
            snap_to_grid: true,
            grid_size: rust_2d_game_engine::render::DEFAULT_GRID_SIZE,
            painting_tiles: false,
            tile_brush: TileBrush::Tile(0),

            selected_item: SelectedItem::None,
            scene_panel_selected_item: ScenePanelSelectedItem::None,

            build_result: Arc::new(Mutex::new(None)),
            is_building: Arc::new(Mutex::new(false)),
            show_build_project_popup: false,

            exit_request: ExitRequest::None,
            open_script_request: None,
            script_insert_request: None,
            save_requested: false,
            build_requested: false,
            clear_save_data_requested: false,
            deleted_file: None,

            undo_stack: UndoStack::new(),
            copied_entity: None,
        }
    }

    /// Make `loaded` the open project: fresh undo history, no selection.
    pub fn open_project(&mut self, loaded: LoadedProject) {
        self.project_name = loaded.metadata.project_name.clone();
        self.project_path = PathBuf::from(&loaded.metadata.project_path);
        self.project_metadata = Some(loaded.metadata);
        self.undo_stack.reset(&loaded.scene_manager);
        self.scene_manager = Some(loaded.scene_manager);
        self.project_open = true;
        self.project_generation += 1;
        self.clear_selection();
        LOGGER.info(format!("Opened project '{}'", self.project_name));
    }

    /// Write the open project to disk. Failures are also reported in the
    /// console.
    pub fn save_project(&self) -> Result<(), String> {
        let (Some(scene_manager), Some(metadata)) = (&self.scene_manager, &self.project_metadata)
        else {
            return Err("No project is open".to_string());
        };
        ProjectManager::save_project_full(&self.project_path, metadata, scene_manager)
            .inspect_err(|e| LOGGER.error(format!("Saving the project failed: {}", e)))
    }

    /// Finish an edit: save the project and record an undo step. Every
    /// completed editor mutation goes through here.
    pub fn commit_change(&mut self) {
        let _ = self.save_project();
        if let Some(scene_manager) = &self.scene_manager {
            self.undo_stack.commit(scene_manager);
        }
    }

    /// Restore the previous committed state (Ctrl+Z / Edit > Undo).
    pub fn undo(&mut self) {
        if let Some(state) = self.undo_stack.undo() {
            self.restore(state);
            LOGGER.info("Undo");
        }
    }

    /// Re-apply the next state (Ctrl+Y, Ctrl+Shift+Z / Edit > Redo).
    pub fn redo(&mut self) {
        if let Some(state) = self.undo_stack.redo() {
            self.restore(state);
            LOGGER.info("Redo");
        }
    }

    fn restore(&mut self, state: SceneManager) {
        self.scene_manager = Some(state);
        // The selection may point at something that no longer exists
        self.clear_selection();
        let _ = self.save_project();
    }

    pub fn select_scene(&mut self, scene_id: Uuid) {
        self.selected_item = SelectedItem::Scene(scene_id);
        self.scene_panel_selected_item = ScenePanelSelectedItem::Scene(scene_id);
    }

    pub fn select_entity(&mut self, scene_id: Uuid, entity_id: Uuid) {
        self.selected_item = SelectedItem::Entity(scene_id, entity_id);
        self.scene_panel_selected_item = ScenePanelSelectedItem::Entity(scene_id, entity_id);
    }

    pub fn clear_selection(&mut self) {
        self.selected_item = SelectedItem::None;
        self.scene_panel_selected_item = ScenePanelSelectedItem::None;
    }

    fn entity(&self, scene_id: Uuid, entity_id: Uuid) -> Option<&Entity> {
        self.scene_manager
            .as_ref()?
            .get_scene(scene_id)?
            .get_entity(entity_id)
            .ok()
    }

    /// Copy an entity: remembered for the Paste menu item, and returned as
    /// text for the system clipboard (see [`crate::clipboard`]).
    pub fn copy_entity(&mut self, scene_id: Uuid, entity_id: Uuid) -> Option<String> {
        let entity = self.entity(scene_id, entity_id)?;
        let text = crate::clipboard::to_clipboard_text(entity, &self.project_path);
        LOGGER.info(format!("Copied '{}'", entity.name));
        self.copied_entity = Some(text.clone());
        Some(text)
    }

    /// Paste the entity last copied in this editor into a scene.
    pub fn paste_copied_entity(&mut self, scene_id: Uuid) -> Option<Uuid> {
        let text = self.copied_entity.as_deref()?;
        let entity = crate::clipboard::from_clipboard_text(text, &self.project_path)?;
        self.paste_entity(scene_id, entity)
    }

    /// Put a copy of an entity into its own scene (see
    /// [`paste_entity`](Self::paste_entity)).
    pub fn duplicate_entity(&mut self, scene_id: Uuid, entity_id: Uuid) -> Option<Uuid> {
        let entity = self.entity(scene_id, entity_id)?.clone();
        self.paste_entity(scene_id, entity)
    }

    /// Add `entity` to a scene with a new id, select it, and record one
    /// undo step. A copy that would exactly cover an entity of the same name
    /// (a duplicate, or a paste into the scene it was copied from) moves by
    /// [`COPY_OFFSET`] until it doesn't, so repeated copies cascade.
    pub fn paste_entity(&mut self, scene_id: Uuid, mut entity: Entity) -> Option<Uuid> {
        let scene = self.scene_manager.as_mut()?.get_scene_mut(scene_id)?;
        let (mut x, mut y) = (entity.get_x(), entity.get_y());
        while scene
            .entities
            .values()
            .any(|other| other.name == entity.name && other.get_x() == x && other.get_y() == y)
        {
            x += COPY_OFFSET;
            y += COPY_OFFSET;
        }
        let _ = entity.set_x(x);
        let _ = entity.set_y(y);
        if entity.get_vector2("position").is_some() {
            let _ = entity.set_or_create_attribute("position", AttributeValue::Vector2(x, y));
        }
        entity.id = Uuid::new_v4();
        let id = entity.id;
        LOGGER.info(format!("Pasted '{}'", entity.name));
        scene.entities.insert(id, entity);
        self.select_entity(scene_id, id);
        self.commit_change();
        Some(id)
    }

    /// The scene a new entity should go into: the selected scene, or the
    /// scene of the selected entity.
    pub fn selected_scene(&self) -> Option<Uuid> {
        match self.scene_panel_selected_item {
            ScenePanelSelectedItem::Scene(scene_id) => Some(scene_id),
            ScenePanelSelectedItem::Entity(scene_id, _) => Some(scene_id),
            ScenePanelSelectedItem::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_stack_semantics() {
        let mut stack = UndoStack::new();

        // State A: empty manager; State B: one scene; State C: two scenes
        let state_a = SceneManager::new();
        let mut state_b = SceneManager::new();
        state_b.create_scene("one").unwrap();
        let mut state_c = state_b.clone();
        state_c.create_scene("two").unwrap();

        stack.reset(&state_a);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());

        stack.commit(&state_b);
        stack.commit(&state_c);
        assert!(stack.can_undo());

        // Undo C -> B
        let restored = stack.undo().expect("undo to B");
        assert_eq!(restored.scenes.len(), 1);
        assert!(stack.can_redo());

        // Undo B -> A
        let restored = stack.undo().expect("undo to A");
        assert_eq!(restored.scenes.len(), 0);
        assert!(!stack.can_undo(), "initial state cannot be undone past");

        // Redo A -> B -> C
        let restored = stack.redo().expect("redo to B");
        assert_eq!(restored.scenes.len(), 1);
        let restored = stack.redo().expect("redo to C");
        assert_eq!(restored.scenes.len(), 2);
        assert!(!stack.can_redo());

        // A new commit clears the redo branch
        stack.undo().expect("back to B");
        stack.commit(&state_c);
        assert!(!stack.can_redo(), "commit must clear redo history");
    }

    #[test]
    fn test_committing_an_unchanged_state_is_a_no_op() {
        // e.g. leaving a text field without changing it, or the same edit
        // being reported twice
        let mut stack = UndoStack::new();
        let mut state = SceneManager::new();
        stack.reset(&state);
        state.create_scene("one").unwrap();
        stack.commit(&state);
        stack.commit(&state);

        stack.undo().expect("one real step");
        assert!(!stack.can_undo(), "the duplicate commit added no step");

        // ...and doesn't wipe the redo history either
        let reset = SceneManager::new();
        stack.commit(&reset);
        assert!(stack.can_redo());
    }
}

use crate::gui::gui_state::{GuiState, SelectedItem};
use crate::logger::LOGGER;
use eframe::egui;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often the cached file tree is refreshed from disk. Imports, builds
/// and external edits show up after at most this delay (or immediately via
/// the refresh button).
const TREE_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

struct FileNode {
    path: PathBuf,
    name: String,
    children: Vec<FileNode>, // populated for directories; sorted dirs-first
    is_dir: bool,
}

pub struct FileSystem {
    search_query: String,
    selected_file: Option<PathBuf>,
    show_search: bool,
    // Cached tree so we don't walk the project directory every frame
    cached_tree: Option<FileNode>,
    cached_root: PathBuf,
    last_scan: Option<Instant>,
    // Set when the user selects a code file; consumed by show()'s return
    pending_open: Option<PathBuf>,
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            selected_file: None,
            show_search: false,
            cached_tree: None,
            cached_root: PathBuf::new(),
            last_scan: None,
            pending_open: None,
        }
    }

    /// Drop the cached tree; it is rebuilt on the next frame.
    pub fn refresh(&mut self) {
        self.cached_tree = None;
        self.last_scan = None;
    }

    pub fn show(
        &mut self,
        _ctx: &egui::Context,
        ui: &mut egui::Ui,
        gui_state: &mut GuiState,
    ) -> Option<(PathBuf, String)> {
        // Always show header with integrated search
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2,
                right: 6,
                top: 6,
                bottom: 0,
            },
            corner_radius: egui::CornerRadius::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Files");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔍").clicked() {
                        self.show_search = !self.show_search;
                        if !self.show_search {
                            self.search_query.clear();
                        }
                    }
                    if ui.button("⟳").on_hover_text("Refresh file tree").clicked() {
                        self.refresh();
                    }

                    if self.show_search {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .desired_width(150.0)
                                .hint_text("Search files..."),
                        );
                    }
                });
            });
            ui.separator();
        });

        // File tree in scrollable area with margin
        egui::Frame {
            inner_margin: egui::Margin::ZERO,
            outer_margin: egui::Margin {
                left: 2,
                right: 2,
                top: 0,
                bottom: 2,
            },
            corner_radius: egui::CornerRadius::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: egui::Color32::TRANSPARENT,
            stroke: egui::Stroke::NONE,
        }
        .show(ui, |ui| {
            if !gui_state.load_project {
                ui.label("No project opened.");
                return;
            }

            self.ensure_tree(&gui_state.project_path.clone());

            let Some(root) = self.cached_tree.take() else {
                ui.label("Failed to read project directory.");
                return;
            };

            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut deleted_any = false;
                    self.render_children(ui, &root, 0, gui_state, &mut deleted_any);
                    if deleted_any {
                        self.refresh();
                    }
                });

            if self.cached_tree.is_none() && self.last_scan.is_some() {
                // Put the tree back unless a refresh was requested
                self.cached_tree = Some(root);
            }
        });

        // Return file content once when the selection changed to a code file
        let path = self.pending_open.take()?;
        match fs::read_to_string(&path) {
            Ok(content) => Some((path, content)),
            Err(e) => {
                LOGGER.error(format!("Failed to open {}: {}", path.display(), e));
                None
            }
        }
    }

    /// (Re)build the cached tree when the project changed, the cache was
    /// invalidated, or the refresh interval elapsed.
    fn ensure_tree(&mut self, project_path: &Path) {
        let stale = self.cached_root != project_path
            || self.cached_tree.is_none()
            || self
                .last_scan
                .is_none_or(|t| t.elapsed() >= TREE_REFRESH_INTERVAL);

        if stale {
            self.cached_tree = Self::scan_dir(project_path, project_path);
            self.cached_root = project_path.to_path_buf();
            self.last_scan = Some(Instant::now());
        }
    }

    fn scan_dir(path: &Path, project_root: &Path) -> Option<FileNode> {
        let entries = fs::read_dir(path).ok()?;

        let mut children: Vec<FileNode> = Vec::new();
        for entry in entries.filter_map(|e| e.ok()) {
            let entry_path = entry.path();

            // Skip build output under the project path
            if entry_path == project_root.join("target") {
                continue;
            }

            if entry_path.is_dir() {
                if let Some(dir_node) = Self::scan_dir(&entry_path, project_root) {
                    children.push(dir_node);
                }
            } else {
                children.push(FileNode {
                    name: entry_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: entry_path,
                    children: Vec::new(),
                    is_dir: false,
                });
            }
        }

        // Directories first, each group sorted by name
        children.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));

        Some(FileNode {
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.to_path_buf(),
            children,
            is_dir: true,
        })
    }

    fn render_children(
        &mut self,
        ui: &mut egui::Ui,
        node: &FileNode,
        depth: usize,
        gui_state: &mut GuiState,
        deleted_any: &mut bool,
    ) {
        let search_query = self.search_query.to_lowercase();
        let is_filtering = !search_query.is_empty();

        for child in &node.children {
            if child.is_dir {
                egui::CollapsingHeader::new(child.name.clone())
                    .default_open(true)
                    .show(ui, |ui| {
                        self.render_children(ui, child, depth + 1, gui_state, deleted_any);
                    });
            } else {
                // Apply search filter to files only
                if is_filtering && !child.name.to_lowercase().contains(&search_query) {
                    continue;
                }

                self.render_file_row(ui, child, depth, gui_state, deleted_any);
            }
        }
    }

    fn render_file_row(
        &mut self,
        ui: &mut egui::Ui,
        file: &FileNode,
        depth: usize,
        gui_state: &mut GuiState,
        deleted_any: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 4.0);

            let selected = self.selected_file.as_ref() == Some(&file.path);
            let response = ui.selectable_label(selected, file.name.clone());

            if response.clicked() && !selected {
                self.selected_file = Some(file.path.clone());
                gui_state.selected_item = SelectedItem::File(file.path.clone());

                // Open code files in the editor (once, on selection change)
                if matches!(
                    file.path.extension().and_then(|e| e.to_str()),
                    Some("rs") | Some("lua")
                ) {
                    self.pending_open = Some(file.path.clone());
                }
            }

            response.context_menu(|ui| {
                if ui.button("Delete").clicked() {
                    if let Err(err) = fs::remove_file(&file.path) {
                        LOGGER.error(format!("Failed to delete file: {}", err));
                    } else {
                        LOGGER.info(format!("Deleted file: {}", file.name));
                        *deleted_any = true;
                        if matches!(&gui_state.selected_item,
                            SelectedItem::File(selected_path)
                            if selected_path == &file.path)
                        {
                            gui_state.selected_item = SelectedItem::None;
                        }
                        if self.selected_file.as_ref() == Some(&file.path) {
                            self.selected_file = None;
                        }
                    }
                    ui.close();
                }
            });
        });
    }
}

pub struct GuiState {
    pub dark_mode: bool,
    pub show_new_project_popup: bool,
    pub show_open_project_popup: bool,
    pub load_project: bool,            // Track if the project should be loaded
    pub project_name: String,          // Store the project name input
    pub project_path: String,          // Store the project path input
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            dark_mode: true,
            show_new_project_popup: false,
            show_open_project_popup: false,
            load_project: false,
            project_name: String::new(),
            project_path: String::new(),
        }
    }

}
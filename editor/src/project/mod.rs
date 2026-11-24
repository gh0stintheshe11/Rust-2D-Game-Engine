//! Editor-side project operations: scaffolding new projects, importing assets
//! and building standalone games. Reading and writing project files lives in
//! the engine (`rust_2d_game_engine::project`).

mod build;
mod import;
mod scaffold;

pub use build::build_project;
pub use import::{AssetType, import_asset};
pub use scaffold::create_project;

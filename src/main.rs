mod gui; // Import the new gui module

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My Egui App",
        options,
        Box::new(|_cc| Box::new(gui::MyApp::default())), // Update to use MyApp from gui module
    )
}
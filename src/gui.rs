use eframe::egui;

// Define your app
#[derive(Default)]
pub struct MyApp { // Make MyApp public
    name: String,
    age: u32,
}

impl eframe::App for MyApp {
    // This function is called every frame
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Welcome to My Egui App!");

            // Input text for name
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name);
            });

            // Slider for age
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));

            // Show a button
            if ui.button("Submit").clicked() {
                println!("Name: {}, Age: {}", self.name, self.age);
            }
        });
    }
}

use eframe::egui;
use journall::journal::Journal;

pub struct JournallApp {}

impl JournallApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {}
    }
}

impl eframe::App for JournallApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Journall");
            ui.add(
                egui::TextEdit::multiline(&mut String::from("Type here (resets every frame)"))
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );
        });
    }
}

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Journall",
        native_options,
        Box::new(|cc| Ok(Box::new(JournallApp::new(cc)))),
    )
}

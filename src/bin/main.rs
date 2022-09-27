use worms::gui::Simulation;

fn main() {
    let options = eframe::NativeOptions {
        // maximized: true,
        // fullscreen: true,
        // resizable: false,
        ..Default::default()
    };

    eframe::run_native(
        "Worms",
        options,
        Box::new(|cc| Box::new(Simulation::new(cc))),
    );
}

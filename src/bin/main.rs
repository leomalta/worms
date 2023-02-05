use worms::gui::SimInterface;

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
        Box::new(|cc| Box::new(SimInterface::new(cc))),
    );
}

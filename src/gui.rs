use crate::{
    composites::{WormBehavior, WormBody},
    config::SimConfig,
    geometry::Point,
    scene::Scene,
};
use eframe::{
    egui::{self, Context},
    epaint::{mutex::Mutex, vec2, CircleShape, Color32, Pos2},
    CreationContext,
};
use std::{ops::DerefMut, sync::Arc};

pub struct Simulation {
    scene: Arc<Mutex<Option<Scene>>>,
    width: f32,
    height: f32,
    config: Option<SimConfig>,
    active: Arc<Mutex<u64>>,
}

impl eframe::App for Simulation {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("Control")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("Reset".to_owned()).clicked() {
                        self.reset_simulation(ctx);
                        if self.active.lock().to_owned() == 0 {
                            self.start(ctx.clone());
                        } else {
                            *self.active.lock().deref_mut() = self.config.as_ref().unwrap().milisec;
                        }
                    }
                    if ui.button("Step".to_owned()).clicked() {
                        if self.scene.lock().is_none() {
                            self.reset_simulation(ctx);
                        }
                        {
                            let mut guard = self.active.lock();
                            *guard.deref_mut() = 0;
                        }
                        if let Some(simulation) = self.scene.lock().as_mut() {
                            simulation.execute();
                        }
                    }
                    if ui.button("Continue".to_owned()).clicked() {
                        if self.active.lock().to_owned() == 0 {
                            self.start(ctx.clone());
                        }
                    }
                })
            });

        // ----------- create the game panel -------------
        egui::CentralPanel::default()
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                self.width = ui.available_width();
                self.height = ui.available_height();
                if let Some(shapes) = self.get_shapes(ui.next_widget_position()) {
                    ui.painter().extend(shapes);
                }
            });

        if let Some(simulation) = self.scene.lock().as_mut() {
            simulation.resize(self.width as usize, self.height as usize);
        }
    }
}

impl Simulation {
    pub fn new(_: &CreationContext) -> Self {
        Self {
            scene: Arc::new(Mutex::new(None)),
            width: f32::default(),
            height: f32::default(),
            config: None,
            active: Arc::new(Mutex::new(0)),
        }
    }

    fn reset_simulation(&mut self, ctx: &egui::Context) {
        let config = SimConfig::from_json("./conf/default.json").unwrap_or_else(|error| {
            println!("{error}");
            egui::Window::new("ERROR!".to_string())
                .frame(egui::Frame::default())
                .enabled(true)
                .show(ctx, |ui| {
                    ui.label(format!("{error}"));
                });
            SimConfig::default()
        });
        self.scene.lock().replace(Scene::new(
            self.width as usize,
            self.height as usize,
            config.scene_params.clone(),
            config.n_worms,
            config.n_rewards,
        ));
        self.config = Some(config);
    }

    fn start(&self, ctx: Context) {
        {
            *self.active.lock().deref_mut() = self.config.as_ref().unwrap().milisec;
        }
        let game = Arc::clone(&self.scene);
        let active = Arc::clone(&self.active);
        std::thread::spawn(move || loop {
            {
                match game.lock().as_mut() {
                    Some(simulation) => simulation.execute(),
                    None => break,
                }
            }
            ctx.request_repaint();
            let timer = active.lock().to_owned();
            if timer == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(timer));
        });
    }

    fn get_shapes(&self, reference: Pos2) -> Option<Vec<egui::Shape>> {
        let mut result: Vec<egui::Shape> = Vec::new();
        for worm in self.scene.lock().as_ref()?.worms() {
            if let Some(shapes) = build_worm(
                worm.1,
                worm.0,
                self.config.as_ref().unwrap().scene_params.body_size,
                reference,
            ) {
                result.extend(shapes.into_iter());
            }
        }
        result.append(&mut build_rewards(
            &self.scene.lock().as_ref()?.rewards(),
            self.config.as_ref().unwrap().scene_params.body_size / 2.,
            reference,
        ));

        Some(result)
    }
}

fn build_worm(
    body: &WormBody,
    behavior: &WormBehavior,
    size: f32,
    reference: Pos2,
) -> Option<Vec<egui::Shape>> {
    let color_scheme = match_color(behavior)?;
    let mut result = Vec::with_capacity(body.size());
    result.push(egui::Shape::Circle(CircleShape {
        center: reference + vec2(body.head().x, body.head().y),
        radius: size,
        fill: color_scheme.0,
        stroke: egui::Stroke::new(1.0, Color32::BLACK),
    }));
    result.extend(body.iter().rev().take(body.size() - 1).map(|point| {
        egui::Shape::Circle(CircleShape {
            center: reference + vec2(point.x, point.y),
            radius: size,
            fill: color_scheme.1,
            stroke: egui::Stroke::new(1.0, Color32::BLACK),
        })
    }));
    Some(result)
}

fn match_color(behavior: &WormBehavior) -> Option<(Color32, Color32)> {
    match behavior {
        WormBehavior::Alive(_) => Some((
            Color32::from_rgb(0x2E, 0xBF, 0xA5), 
            Color32::from_rgb(0x7D, 0xDE, 0x92),
        )),
        WormBehavior::Dead(_) => Some((Color32::GRAY, Color32::from_rgb(0x4E, 0x41, 0x87))),
        WormBehavior::Chasing => Some((
            Color32::from_rgb(0x2E, 0xBF, 0xA5),
            Color32::from_rgb(0x30, 0x83, 0xDC),
        )),
        WormBehavior::Removed => None,
    }
}

fn build_rewards(points: &Vec<Point>, size: f32, reference: Pos2) -> Vec<egui::Shape> {
    points
        .iter()
        .map(|point| {
            egui::Shape::Circle(CircleShape {
                center: reference + vec2(point.x, point.y),
                radius: size,
                fill: Color32::from_rgb(0xF8, 0xFF, 0xE5),
                stroke: egui::Stroke::new(1.0, Color32::BLACK),
            })
        })
        .collect()
}

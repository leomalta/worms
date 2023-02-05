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

pub struct SimInterface {
    config: Option<SimConfig>,
    scene: Arc<Mutex<Option<Scene>>>,
    tick_interval: Arc<Mutex<u64>>,
    width: f32,
    height: f32,
}

impl eframe::App for SimInterface {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        // ----------- create the control bar -------------
        egui::TopBottomPanel::bottom("Control")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if ui.button("Reset".to_owned()).clicked() {
                        // create new simulation
                        self.reset_simulation();
                        // if sim stopped (interval == 0), start sim
                        if self.tick_interval.lock().to_owned() == 0 {
                            self.start(ctx.clone());
                        }
                    }
                    if ui.button("Step".to_owned()).clicked() {
                        // stop the simulation
                        *self.tick_interval.lock().deref_mut() = 0;
                        // if there is a simulation instantiate, execute one step
                        let has_simulation = self
                            .scene
                            .lock()
                            .as_mut()
                            .map(|scene_sim| scene_sim.execute())
                            .is_none();
                        // if no simulation, create a new one
                        if has_simulation {
                            self.reset_simulation();
                        }
                    }
                    // if sim stopped, start it
                    if ui.button("Continue".to_owned()).clicked()
                        && self.tick_interval.lock().to_owned() == 0
                    {
                        self.start(ctx.clone());
                    }
                })
            });

        // ----------- create the game panel -------------
        egui::CentralPanel::default()
            .frame(egui::Frame::canvas(&ctx.style()))
            .show(ctx, |ui| {
                self.width = ui.available_width();
                self.height = ui.available_height();
                let shapes = self.get_shapes(ui.next_widget_position());
                ui.painter().extend(shapes);
            });

        if let Some(simulation) = self.scene.lock().as_mut() {
            simulation.resize(self.width as usize, self.height as usize);
        }
    }
}

impl SimInterface {
    pub fn new(_: &CreationContext) -> Self {
        Self {
            config: None,
            scene: Arc::new(Mutex::new(None)),
            tick_interval: Arc::new(Mutex::new(0)),
            width: f32::default(),
            height: f32::default(),
        }
    }

    pub fn from(scene: Scene) -> Self {
        Self {
            config: None,
            scene: Arc::new(Mutex::new(Some(scene))),
            tick_interval: Arc::new(Mutex::new(0)),
            width: f32::default(),
            height: f32::default(),
        }
    }

    fn reset_simulation(&mut self) {
        // Read the default configuration
        let new_config = SimConfig::read_default();

        // Build the new_scene using the config read
        let mut new_scene = Scene::new(
            self.width as usize,
            self.height as usize,
            new_config.scene_params.clone(),
            new_config.n_worms,
            new_config.n_rewards,
        );
        for _ in 0..50 {
            new_scene.execute();
        }

        // Update the internal attributes
        self.config = Some(new_config);
        self.scene.lock().replace(new_scene);
    }

    fn start(&self, ctx: Context) {
        // Set the time interval for each simulatiom tick, if there is a config object set
        self.config
            .as_ref()
            .map(|config| *self.tick_interval.lock().deref_mut() = config.interval);

        // Start the thread for simulation
        let scene = Arc::clone(&self.scene);
        let interval = Arc::clone(&self.tick_interval);
        std::thread::spawn(move || {
            loop {
                // tick the simulation
                let result = tick_simulation(scene.as_ref(), interval.as_ref());
                // repaint
                ctx.request_repaint();
                // wait interval
                match result {
                    Some(timer) => std::thread::sleep(std::time::Duration::from_millis(timer)),
                    None => break,
                }
            }
        });
    }

    pub fn get_shapes(&self, reference: Pos2) -> Vec<egui::Shape> {
        let size = self
            .config
            .as_ref()
            .map(|config| config.scene_params.body_size)
            .unwrap_or_default();
        self.scene
            .lock()
            .as_ref()
            .map(|scene_sim| {
                scene_sim
                    .worms()
                    .flat_map(|(behavior, body)| build_worm(body, behavior, size, reference))
                    .chain(build_rewards(scene_sim.rewards(), size / 2., reference))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn tick_simulation(scene: &Mutex<Option<Scene>>, active_timer: &Mutex<u64>) -> Option<u64> {
    scene
        .lock()
        .as_mut()
        .map(|scene_sim| {
            scene_sim.execute();
            let timer = active_timer.lock().clone();
            (timer != 0).then_some(timer)
        })
        .flatten()
}

// Return an iterator over the shapes from the body of a
fn build_worm<'a>(
    body: &'a WormBody,
    behavior: &'a WormBehavior,
    size: f32,
    reference: Pos2,
) -> impl Iterator<Item = egui::Shape> + 'a {
    // get the color of the head and body
    let (head_color, body_color) = match_color(behavior).unwrap_or((Color32::WHITE, Color32::RED));
    // create the head
    body.iter()
        .take(1)
        .map(move |point| {
            CircleShape::filled(reference + vec2(point.x, point.y), size, head_color).into()
        })
        // chain the rest of the body
        .chain(body.iter().rev().take(body.size().saturating_sub(1)).map(move |point| {
            CircleShape::filled(reference + vec2(point.x, point.y), size, body_color).into()
        }))
}

fn build_rewards(
    points: &[Point],
    size: f32,
    reference: Pos2,
) -> impl Iterator<Item = egui::Shape> + '_ {
    let reward_color = Color32::from_rgb(0xF8, 0xFF, 0xE5);
    points.iter().map(move |point| {
        CircleShape::filled(reference + vec2(point.x, point.y), size, reward_color).into()
    })
}

fn match_color(behavior: &WormBehavior) -> Option<(Color32, Color32)> {
    let moving_head_color = Color32::from_rgb(0x2E, 0xBF, 0xA5);
    match behavior {
        WormBehavior::Alive(_) => Some((moving_head_color, Color32::from_rgb(0x7D, 0xDE, 0x92))),
        WormBehavior::Dead(_) => Some((Color32::GRAY, Color32::from_rgb(0x4E, 0x41, 0x87))),
        WormBehavior::Chasing => Some((moving_head_color, Color32::from_rgb(0x30, 0x83, 0xDC))),
        WormBehavior::Removed => None,
    }
}

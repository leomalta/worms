use crate::composites::*;
use crate::geometry::{Direction, Point};
use crate::movement::*;
use rayon::prelude::*;

#[derive(Clone)]
pub struct SceneParameters {
    pub worm_size: usize,
    pub body_size: f32,
    pub starvation: usize,
    pub expiration: usize,
}

struct SceneContent {
    behaviors: Vec<WormBehavior>,
    movements: Vec<Movement>,
    bodies: Vec<WormBody>,
    rewards: Vec<Reward>,
    reward_destination: Vec<Point>,
}

impl SceneContent {
    fn rand(
        n_worms: usize,
        n_rewards: usize,
        worm_size: usize,
        body_size: f32,
        width: usize,
        height: usize,
    ) -> Self {
        let behaviors = vec![WormBehavior::Alive(0); n_worms];
        let mut movements = Vec::with_capacity(n_worms);
        let mut bodies = Vec::with_capacity(n_worms);

        for _ in 0..n_worms {
            let worm = Worm::rand(worm_size, body_size, width, height);
            movements.push(worm.movement);
            bodies.push(worm.body);
        }

        let mut rewards = Vec::with_capacity(n_rewards);
        let mut reward_destination = Vec::with_capacity(n_rewards);
        for _ in 0..n_rewards {
            rewards.push(Reward::rand(width, height));
            reward_destination.push(Point::rand(width, height));
        }
        Self {
            behaviors,
            movements,
            bodies,
            rewards,
            reward_destination,
        }
    }
}

pub struct Scene {
    width: usize,
    height: usize,
    params: SceneParameters,
    stats: WormStats,
    content: SceneContent,
}

impl Scene {
    pub fn new(
        width: usize,
        height: usize,
        params: SceneParameters,
        n_worms: usize,
        n_rewards: usize,
    ) -> Self {
        Self {
            width,
            height,
            stats: WormStats::default(),
            content: SceneContent::rand(
                n_worms,
                n_rewards,
                params.worm_size,
                params.body_size,
                width,
                height,
            ),
            params,
        }
    }

    pub fn worms<'a>(&'a self) -> impl Iterator<Item = (&WormBehavior, &WormBody)> {
        self.content
            .behaviors
            .iter()
            .zip(self.content.bodies.iter())
    }

    pub fn rewards<'a>(&'a self) -> &'a Vec<Reward> {
        &self.content.rewards
    }

    pub fn movements<'a>(&'a self) -> &'a Vec<Movement> {
        &self.content.movements
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }

    pub fn execute(&mut self) {
        self.update_worms();
        self.update_rewards();
    }

    fn update_worms(&mut self) {
        for worm_id in 0..self.content.behaviors.len() {
            match self.content.behaviors[worm_id] {
                WormBehavior::Alive(counter) => {
                    if self.content.bodies[worm_id].full() {
                        self.content.behaviors[worm_id] = self.split_worm(worm_id);
                    } else {
                        self.content.behaviors[worm_id] = self.execute_alive(worm_id, counter)
                    }
                }

                WormBehavior::Dead(counter) => {
                    if counter < self.params.expiration {
                        self.content.behaviors[worm_id] = WormBehavior::Dead(counter + 1)
                    } else {
                        self.content.behaviors[worm_id] = WormBehavior::Removed;
                        self.content.bodies[worm_id].set_size(0)
                    }
                }

                WormBehavior::Chasing => {
                    let result = self.execute_chasing(worm_id);
                    self.content.behaviors[worm_id] = match result {
                        WormBehavior::Chasing => self.execute_chasing(worm_id),
                        _ => result,
                    };
                }

                WormBehavior::Removed => (),
            }
        }
    }

    fn update_rewards(&mut self) {
        for i in 0..self.content.reward_destination.len() {
            if self.content.reward_destination[i].distance_to(&self.content.rewards[i])
                < self.params.body_size
            {
                self.content.reward_destination[i] = Point::rand(self.width, self.height);
            }
            let direction =
                self.content.rewards[i].direction_to(&self.content.reward_destination[i]);
            self.content.rewards[i] =
                self.content.rewards[i].copy(direction, self.params.body_size / 4.);
        }
    }

    fn execute_alive(&mut self, worm_id: usize, counter: usize) -> WormBehavior {
        if self.content.bodies[worm_id].full() {
            return self.split_worm(worm_id);
        }

        let mover = AliveWormMover {
            rewards: &self.content.rewards,
            bodies: &self.content.bodies,
        };

        match execute_movement(
            &self.content.movements[worm_id],
            &mover,
            &self.stats,
            self.width,
            self.height,
            self.params.body_size * 2.,
        ) {
            MovementResult::TargetHit(target_index, movement) => {
                self.content.rewards[target_index] = Reward::rand(self.width, self.height);
                self.content.bodies[worm_id].grow(movement.origin);
                self.content.movements[worm_id] = movement;
                return WormBehavior::Alive(0);
            }
            MovementResult::TargetMiss(movement) => {
                self.content.bodies[worm_id].roll(movement.origin);
                self.content.movements[worm_id] = movement;
                if counter < self.params.starvation / self.content.bodies[worm_id].size() {
                    return WormBehavior::Alive(counter + 1);
                }
                return WormBehavior::Chasing;
            }
            MovementResult::None => return WormBehavior::Dead(0),
        }
    }

    fn execute_chasing(&mut self, worm_id: usize) -> WormBehavior {
        let mover = ChasingWormMover {
            rewards: &self.content.rewards,
            bodies: &self.content.bodies,
            behaviors: &self.content.behaviors,
        };

        match execute_movement(
            &self.content.movements[worm_id],
            &mover,
            &self.stats,
            self.width,
            self.height,
            self.params.body_size * 2.,
        ) {
            MovementResult::TargetHit(target_index, _) => {
                self.merge_worms(worm_id, target_index);
                return WormBehavior::Alive(0);
            }
            MovementResult::TargetMiss(movement) => {
                self.content.bodies[worm_id].roll(movement.origin);
                self.content.movements[worm_id] = movement;
                return WormBehavior::Chasing;
            }
            MovementResult::None => return WormBehavior::Dead(0),
        }
    }

    fn next_free_index(&mut self) -> usize {
        if let Some(index) =
            self.content
                .behaviors
                .par_iter()
                .position_any(|behavior| match behavior {
                    WormBehavior::Removed => true,
                    _ => false,
                })
        {
            return index;
        }
        self.content.bodies.push(WormBody::default());
        self.content.behaviors.push(WormBehavior::Removed);
        self.content.movements.push(Movement::default());
        self.content.bodies.len() - 1
    }

    fn split_worm(&mut self, worm_id: usize) -> WormBehavior {
        while self.content.bodies[worm_id].size() >= self.params.worm_size * 2 {
            let size_after_split = self.content.bodies[worm_id].size() - self.params.worm_size;

            let free_index = self.next_free_index();

            for part in self.content.bodies[worm_id]
                .iter()
                .rev()
                .take(self.params.worm_size)
                .cloned()
                .collect::<Vec<_>>()
            {
                self.content.bodies[free_index].grow(part);
            }

            self.content.behaviors[free_index] = WormBehavior::Alive(0);
            self.content.movements[free_index].destination = None;
            self.content.movements[free_index].origin =
                self.content.bodies[free_index].head().clone();

            self.content.bodies[worm_id].set_size(size_after_split);
        }
        WormBehavior::Alive(0)
    }

    fn merge_worms(&mut self, worm_id: usize, other_pos: usize) {
        let other_size = self.content.bodies[other_pos].size();
        let diff = *self.content.bodies[other_pos].tail() - *self.content.bodies[worm_id].head();

        self.content.bodies[worm_id].shrink(1);
        self.content.bodies[worm_id].shift(diff);

        let transfer = self.content.bodies[other_pos]
            .iter()
            .rev()
            .take(self.content.bodies[worm_id].available_space())
            .cloned()
            .collect::<Vec<_>>();

        let removed = transfer.len();
        for part in transfer {
            self.content.bodies[worm_id].grow(part);
        }

        self.content.bodies[other_pos].set_size(other_size - removed);
        if other_size - removed == 0 {
            self.content.behaviors[other_pos] = WormBehavior::Removed
        }

        self.content.movements[worm_id] = Movement {
            origin: self.content.bodies[worm_id].head().clone(),
            destination: None,
            direction: Direction::rand(),
        };
    }
}

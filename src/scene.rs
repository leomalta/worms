use crate::composites::*;
use crate::geometry::Point;
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
        let bodies = (0..n_worms)
            .into_iter()
            .map(|_| WormBody::rand(worm_size, body_size, width, height))
            .collect::<Vec<_>>();
        let rewards = (0..n_rewards)
            .into_iter()
            .map(|_| Reward::rand(width, height))
            .collect::<Vec<_>>();
        let reward_destination = (0..n_rewards)
            .into_iter()
            .map(|_| Point::rand(width, height))
            .collect::<Vec<_>>();

        Self {
            behaviors,
            bodies,
            rewards,
            reward_destination,
        }
    }
}

pub struct Scene {
    params: SceneParameters,
    width: usize,
    height: usize,
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

    pub fn worms(&self) -> impl Iterator<Item = (&WormBehavior, &WormBody)> {
        self.content
            .behaviors
            .iter()
            .zip(self.content.bodies.iter())
    }

    pub fn rewards(&self) -> &[Reward] {
        &self.content.rewards
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

    /// Move the rewards in the scene
    fn update_rewards(&mut self) {
        for i in 0..self.content.reward_destination.len() {
            let direction =
                self.content.rewards[i].direction_to(self.content.reward_destination[i]);

            let new_reward = self.content.rewards[i].copy(direction, self.params.body_size / 4.);

            let is_valid = new_reward.x <= self.width as f32
                && new_reward.y <= self.height as f32
                && self.content.reward_destination[i].distance_to(new_reward)
                    >= self.params.body_size;

            self.content.rewards[i] = is_valid
                .then_some(new_reward)
                .unwrap_or(Point::rand(self.width, self.height));
        }
    }

    fn execute_alive(&mut self, worm_id: usize, counter: usize) -> WormBehavior {
        let mover = AliveWormMover {
            details: &self.get_movement_details(worm_id),
            rewards: &self.content.rewards,
            bodies: &self.content.bodies,
        };

        match mover.execute_movement(self.params.body_size * 2.) {
            MovementResult::TargetHit(target_index, new_head) => {
                self.content.rewards[target_index] = Reward::rand(self.width, self.height);
                self.content.bodies[worm_id].grow(new_head);
                WormBehavior::Alive(0)
            }
            MovementResult::TargetMiss(new_head, destination) => {
                self.content.bodies[worm_id].roll(new_head, destination);
                if counter < self.params.starvation / self.content.bodies[worm_id].size() {
                    return WormBehavior::Alive(counter + 1);
                }
                WormBehavior::Chasing
            }
            MovementResult::None => WormBehavior::Dead(0),
        }
    }

    fn execute_chasing(&mut self, worm_id: usize) -> WormBehavior {
        let mover = ChasingWormMover {
            details: &self.get_movement_details(worm_id),
            rewards: &self.content.rewards,
            bodies: &self.content.bodies,
            behaviors: &self.content.behaviors,
        };

        match mover.execute_movement(self.params.body_size * 2.) {
            MovementResult::TargetHit(target_index, _) => {
                self.merge_worms(worm_id, target_index);
                WormBehavior::Alive(0)
            }
            MovementResult::TargetMiss(new_head, destination) => {
                self.content.bodies[worm_id].roll(new_head, destination);
                WormBehavior::Chasing
            }
            MovementResult::None => WormBehavior::Dead(0),
        }
    }

    fn get_movement_details(&self, worm_id: usize) -> MovementDetails {
        MovementDetails {
            origin: *self.content.bodies[worm_id].head(),
            chosen_destination: self.content.bodies[worm_id].target,
            stats: self.stats,
            width: self.width,
            height: self.height,
        }
    }

    /// Return the index of the first worm having the Removed behavior
    /// Creates a new worm if none is found
    fn next_removed_index(&mut self) -> usize {
        self.content
            .behaviors
            .par_iter()
            .position_any(|behavior| matches!(behavior, WormBehavior::Removed))
            .unwrap_or_else(|| {
                self.content.bodies.push(WormBody::default());
                self.content.behaviors.push(WormBehavior::Removed);
                self.content.bodies.len() - 1
            })
    }

    fn split_worm(&mut self, worm_id: usize) -> WormBehavior {
        // While the worm has a size that can be split
        while self.content.bodies[worm_id].size() >= self.params.worm_size * 2 {
            // Calculate the new size after the split
            let size_after_split = self.content.bodies[worm_id].size() - self.params.worm_size;
            // Get the first index of a content table entry that is free (i.e has a removed worm)
            let free_index = self.next_removed_index();
            // activate the worm at the found free_index
            self.content.behaviors[free_index] = WormBehavior::Alive(0);
            // Copy all the desired parts to the body in the free_index
            self.content.bodies[worm_id]
                .iter()
                .rev()
                .take(self.params.worm_size)
                .cloned()
                .collect::<Vec<_>>()
                .iter()
                .fold(&mut self.content.bodies[free_index], |acc, &part| {
                    acc.grow(part);
                    acc
                });
            // Reduce the size of the worm after the split
            self.content.bodies[worm_id].set_size(size_after_split);
        }
        WormBehavior::Alive(0)
    }

    fn merge_worms(&mut self, worm_id: usize, target_id: usize) {
        // Remove the head of the worm
        self.content.bodies[worm_id].shrink(1);
        // Calculate the gap between the head of the worm and the tail of the target worm
        let diff = *self.content.bodies[target_id].tail() - *self.content.bodies[worm_id].head();
        // Align the rest of worm body to the 'target' worm body
        self.content.bodies[worm_id].shift(diff);
        // Store the original size of the worm
        let original_worm_size = self.content.bodies[worm_id].size();

        // Copy all the parts that fit to the original worm
        self.content.bodies[target_id]
            .iter()
            .rev()
            .take(self.content.bodies[worm_id].available_space())
            .cloned()
            .collect::<Vec<_>>()
            .iter()
            .fold(&mut self.content.bodies[worm_id], |acc, &part| {
                acc.grow(part);
                acc
            });

        // Get the new size of the target worm (subtracting the transfered parts)
        let removed = self.content.bodies[worm_id].size() - original_worm_size;
        let target_worm_size = self.content.bodies[target_id].size() - removed;

        // Remove the copied parts from the 'target' by reducing its size
        self.content.bodies[target_id].set_size(target_worm_size);
        if target_worm_size == 0 {
            self.content.behaviors[target_id] = WormBehavior::Removed
        }
    }
}

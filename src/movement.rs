use crate::{
    composites::{Reward, WormBehavior, WormBody, WormPart, WormStats},
    geometry::{Direction, Point, Rotator},
};
use rayon::prelude::*;

#[derive(Default)]
pub struct Movement {
    pub origin: Point,
    pub destination: Option<Point>,
}

impl Movement {
    fn direction(&self) -> Direction {
        match self.destination {
            Some(destination) => self.origin.direction_to(&destination),
            None => Direction::rand(),
        }
    }

    /// Checks if a given destination is in range of the movement
    /// (according to its origin and direction)
    pub fn in_range(&self, destination: &Point, stats: &WormStats) -> bool {
        self.direction()
            .connect(&self.origin, destination, stats.vision_range)
            && self.origin.distance_to(destination) < stats.vision_distance
    }
}

/// Struct to represent a valid movement target. i.e a target contained in another composite
/// it contains the index of the composite containing the target
/// the distance to the target
/// and the target itself
struct ValidTarget {
    target_id: usize,
    distance: f32,
    target: Point,
}

pub trait Mover {
    /// Chooses a target to follow as movement destination
    /// Returns the index of the composite containing the target, if any, and the chosen target
    fn select(
        &self,
        movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point);

    /// Checks if a given worm part does not collide (i.e is at least a given distance from all the obstacles)
    fn collides(&self, part: &WormPart, distance: f32) -> bool;
}

/// Mover for the 'Alive' worm
/// holds the refereces to candidate targets: the rewards
/// and the obstacles: other snakes
pub struct AliveWormMover<'a> {
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
}

/// Mover for the 'Alive' worm
/// holds the refereces to candidate targets: other 'alive' snakes
/// and the obstacles: other snakes not alive and rewards
pub struct ChasingWormMover<'a> {
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
    pub behaviors: &'a Vec<WormBehavior>,
}

impl Mover for AliveWormMover<'_> {
    /// Search for a Reward to reach
    fn select(
        &self,
        saved_movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point) {
        match self
            .rewards
            .par_iter()
            .enumerate()
            // Filter the rewards in range (according to the vision stats)
            .filter(|(_, target)| saved_movement.in_range(target, stats))
            // map the reward as a ValidTarget
            .map(|(pos, target)| ValidTarget {
                target_id: pos,
                distance: saved_movement.origin.distance_to(&target),
                target: target.clone(),
            })
            // choose the closest one
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance))
        {
            Some(chosen_target) => (Some(chosen_target.target_id), chosen_target.target),
            None => (None, recovery_target(width, height, saved_movement, stats)),
        }
    }

    fn collides(&self, part: &WormPart, distance: f32) -> bool {
        self.bodies.par_iter().any(|body| {
            body.iter()
                .any(|point| point.distance_to(&part) < distance - 0.01)
        })
    }
}

impl Mover for ChasingWormMover<'_> {
    fn select(
        &self,
        saved_movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point) {
        match self
            .bodies
            .par_iter()
            .enumerate()
            // Filter the snakes alive and in range
            .filter(|(pos, target)| match self.behaviors[*pos] {
                WormBehavior::Alive(_) => saved_movement.in_range(target.tail(), stats),
                _ => false,
            })
            // map the snake tail as a ValidTarget
            .map(|(pos, target)| ValidTarget {
                target_id: pos,
                distance: saved_movement.origin.distance_to(target.tail()),
                target: target.tail().clone(),
            })
            // choose the closest one
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance))
        {
            Some(chosen_target) => (Some(chosen_target.target_id), chosen_target.target),
            None => (None, recovery_target(width, height, saved_movement, stats)),
        }
    }

    fn collides(&self, part: &WormPart, distance: f32) -> bool {
        self.bodies
            .par_iter()
            .enumerate()
            .any(|(pos, body)| match self.behaviors[pos] {
                // Skip the tail of alive snakes as they are valid targets
                WormBehavior::Alive(_) => body
                    .iter()
                    .take(body.size() - 1)
                    .any(|point| point.distance_to(&part) < distance - 0.1),
                _ => body
                    .iter()
                    .any(|point| point.distance_to(&part) < distance - 0.1),
            })
            || self
                .rewards
                .par_iter()
                .any(|point| point.distance_to(&part) < distance - 0.1)
    }
}

/// Checks if the saved_movement destination is still valid and returns it,
/// Otherwise generates a new one
fn recovery_target(width: usize, height: usize, saved_movement: &Movement, stats: &WormStats) -> Point {
    match saved_movement.destination {
        Some(destination) if saved_movement.origin.distance_to(&destination) > stats.vision_distance => {
            destination
        }
        _ => Point::rand(width, height),
    }
}

pub enum MovementResult {
    TargetHit(usize, Movement),
    TargetMiss(Movement),
    None,
}

/// Function to execute a movement: it gets a saved_movement and a Mover impl
/// Returns a MovementResult enum to indicate the action to be taken
pub fn execute_movement(
    saved_movement: &Movement,
    mover: &dyn Mover,
    stats: &WormStats,
    width: usize,
    height: usize,
    distance: f32,
) -> MovementResult {
    let (pos, destination) = mover.select(saved_movement, stats, width, height);

    // create the rotator starting from the movement direction
    let mut rotator = Rotator::new(saved_movement.origin.direction_to(&destination));

    while let Some(direction) = rotator.next() {
        // create the new worm head
        let new_head = saved_movement.origin.copy(direction, distance);

        // if head do not collide with others, set the movement origin as the new head
        if !mover.collides(&new_head, distance) {
            let selected_movement = Movement {
                origin: new_head,
                destination: Some(destination),
            };
            match pos.and(selected_movement.destination) {
                Some(destination)
                if destination.distance_to(&selected_movement.origin) < distance =>
                {
                    // If the destination is reached with the new head, target is hit
                    return MovementResult::TargetHit(pos.unwrap(), selected_movement)
                }
                _ => return MovementResult::TargetMiss(selected_movement),
            }
        }
    }
    MovementResult::None
}

use crate::{
    composites::{Reward, WormBehavior, WormBody, WormPart, WormStats},
    geometry::{Direction, Point, Rotator},
};
use rayon::prelude::*;

#[derive(Default)]
pub struct Movement {
    pub origin: Point,
    pub destination: Option<Point>,
    pub direction: Direction,
}

impl Movement {
    pub fn in_range(&self, destination: &Point, stats: &WormStats) -> bool {
        self.direction
            .connect(&self.origin, destination, stats.vision_range)
            && self.origin.distance_to(destination) < stats.vision_distance
    }
}

struct ChosenTarget {
    target_id: usize,
    distance: f32,
    target: Point,
}

pub trait Mover {
    fn select(
        &self,
        movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point);
    fn collides(&self, part: &WormPart, distance: f32) -> bool;
}

pub struct AliveWormMover<'a> {
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
}

pub struct ChasingWormMover<'a> {
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
    pub behaviors: &'a Vec<WormBehavior>,
}

impl Mover for AliveWormMover<'_> {
    fn select(
        &self,
        saved_movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point) {
        let chosen_target = self
            .rewards
            .par_iter()
            .enumerate()
            .filter(|(_, target)| saved_movement.in_range(target, stats))
            .map(|(pos, target)| ChosenTarget {
                target_id: pos,
                distance: saved_movement.origin.distance_to(&target),
                target: target.clone(),
            })
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance));
        adjust_target(width, height, saved_movement, stats, chosen_target)
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
        movement: &Movement,
        stats: &WormStats,
        width: usize,
        height: usize,
    ) -> (Option<usize>, Point) {
        let choice = self
            .bodies
            .par_iter()
            .enumerate()
            .filter(|(pos, target)| match self.behaviors[*pos] {
                WormBehavior::Alive(_) => movement.in_range(target.tail(), stats),
                _ => false,
            })
            .map(|(pos, target)| ChosenTarget {
                target_id: pos,
                distance: movement.origin.distance_to(target.tail()),
                target: target.tail().clone(),
            })
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance));
        adjust_target(width, height, movement, stats, choice)
    }

    fn collides(&self, part: &WormPart, distance: f32) -> bool {
        self.bodies
            .par_iter()
            .enumerate()
            .any(|(pos, body)| match self.behaviors[pos] {
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

fn adjust_target(
    width: usize,
    height: usize,
    movement: &Movement,
    stats: &WormStats,
    chosen_target: Option<ChosenTarget>,
) -> (Option<usize>, Point) {
    match chosen_target {
        Some(chosen) => (Some(chosen.target_id), chosen.target),
        _ => match movement.destination {
            Some(destination)
                if movement.origin.distance_to(&destination) > stats.vision_distance =>
            {
                (None, destination)
            }
            _ => (None, Point::rand(width, height)),
        },
    }
}

pub enum MovementResult {
    TargetHit(usize, Movement),
    TargetMiss(Movement),
    None,
}
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
                direction,
                origin: new_head,
                destination: Some(destination),
            };
            match pos.and(selected_movement.destination) {
                Some(destination)
                    if destination.distance_to(&selected_movement.origin) < distance =>
                {
                    return MovementResult::TargetHit(pos.unwrap(), selected_movement)
                }
                _ => return MovementResult::TargetMiss(selected_movement),
            }
        }
    }
    MovementResult::None
}

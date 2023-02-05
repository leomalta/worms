use crate::{
    composites::{Reward, WormBehavior, WormBody, WormPart, WormStats},
    geometry::{Point, Rotator},
};
use rayon::prelude::*;

// Struct with the data needed to calculate the movement of a worm
pub struct MovementDetails {
    pub origin: WormPart,
    pub chosen_destination: Point,
    pub stats: WormStats,
    pub width: usize,
    pub height: usize,
}

impl MovementDetails {
    /// Returns the current chosen destination if it is OUTSIDE vision range
    /// or a randon Point otherwise
    fn choose_destination(&self) -> Point {
        (self.origin.distance_to(self.chosen_destination) > self.stats.vision_distance)
            .then_some(self.chosen_destination)
            .unwrap_or(Point::rand(self.width, self.height))
    }

    fn is_inside_area(&self, new_head: WormPart) -> bool {
        new_head.x <= self.width as f32 && new_head.y <= self.height as f32
    }
}

// Enum to represent the result of a movement attempt
pub enum MovementResult {
    // Movement hits the target:
    // (index of the composite having the target, target itself)
    TargetHit(usize, WormPart),
    // Movement misses the target:
    // (new head after movement, chosen destination to follow)
    TargetMiss(WormPart, Point),
    // Movement not possible
    None,
}

pub trait Mover {
    /// Chooses a target to follow as movement destination
    /// Returns the index of the composite containing the target, if any, and the chosen target
    fn select_target(&self) -> (Option<usize>, Point);

    /// Checks if a given worm part does not collide (i.e is at least a given distance from all the obstacles)
    fn collides(&self, part: WormPart, distance: f32) -> bool;

    fn origin(&self) -> WormPart;

    fn details(&self) -> &MovementDetails;

    /// Function to execute a movement: it gets a saved_movement and a Mover impl
    /// Returns a MovementResult enum to indicate the action to be taken
    fn execute_movement(&self, distance: f32) -> MovementResult {
        // select the id of the target and the desired point position to follow
        let (target_id, destination) = self.select_target();

        // iterate over the all possible directions (choosing the ones closest to the target first)
        Rotator::new(self.origin().direction_to(destination))
            // get a new head in a direction that do no collide with anything
            .find_map(|direction| {
                // create the new_head pointing in the iterated direction
                let new_head = self.origin().copy(direction, distance);
                // return Some(new_head) if the head do not collide with any obstable
                let is_valid = self.details().is_inside_area(new_head) && !self.collides(new_head, distance);
                is_valid.then_some(new_head)
            })
            .and_then(|valid_head| {
                // If the destination is reached with the new head, some target is hit
                (destination.distance_to(valid_head) < distance)
                    // if the target is part of a composite (i.e has a target_id)
                    // return the id of the target hit and the new head created
                    .then_some(target_id.map(|id| MovementResult::TargetHit(id, valid_head)))
                    // otherwise, destination not reached
                    .unwrap_or(Some(MovementResult::TargetMiss(valid_head, destination)))
            })
            // No valid movement could be found
            .unwrap_or(MovementResult::None)
    }
}

/// Struct to represent a valid movement target. i.e a position contained in another composite
/// it contains the index of the composite containing the target
/// the distance to the target
/// and the target itself
struct ValidTarget {
    target_id: usize,
    target: Point,
    distance: f32,
}

impl ValidTarget {
    fn from(origin: WormPart, target_id: usize, target: Point) -> Self {
        Self {
            target_id,
            target,
            distance: origin.distance_to(target),
        }
    }
}

/// Mover for the 'Alive' worm
/// holds the refereces to candidate targets: the rewards
/// and the obstacles: the other worm bodies
pub struct AliveWormMover<'a> {
    pub details: &'a MovementDetails,
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
}

impl<'a> AliveWormMover<'a> {
    // Converts a reward into a ValidTarget if it is in vision range
    fn to_valid_target(&self, id: usize, reward: Reward) -> Option<ValidTarget> {
        in_range(
            self.details.origin,
            self.details.chosen_destination,
            reward,
            &self.details.stats,
        )
        .then_some(ValidTarget::from(self.details.origin, id, reward))
    }
}

impl Mover for AliveWormMover<'_> {
    fn origin(&self) -> WormPart {
        self.details.origin
    }

    fn details(&self) -> &MovementDetails {
        self.details
    }

    /// Search for the closest reward in the visible range
    /// Return the index of the reward in the table (if any) and its position
    /// (or a randon one if no reward found)
    fn select_target(&self) -> (Option<usize>, Point) {
        match self
            .rewards
            .par_iter()
            .enumerate()
            // Filter the rewards in vision range, mapping them as a ValidTarget
            .filter_map(|(rwd_id, &rwd)| self.to_valid_target(rwd_id, rwd))
            // choose the closest ValidTarget
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance))
        {
            Some(closest_valid) => (Some(closest_valid.target_id), closest_valid.target),
            // No valid target found, returns the destination according to the movement details
            None => (None, self.details.choose_destination()),
        }
    }

    fn collides(&self, part: WormPart, distance: f32) -> bool {
        // iterates over all the parts of all worm bodies, checking for collision
        self.bodies.par_iter().any(|body| {
            body.iter()
                .any(|point| point.distance_to(part) < distance - 0.01)
        })
    }
}

/// Mover for the 'Chasing' worm
/// holds the refereces to candidate targets: other 'alive' snakes
/// and the obstacles: other snakes not alive and rewards
pub struct ChasingWormMover<'a> {
    pub details: &'a MovementDetails,
    pub rewards: &'a Vec<Reward>,
    pub bodies: &'a Vec<WormBody>,
    pub behaviors: &'a Vec<WormBehavior>,
}

impl<'a> ChasingWormMover<'a> {
    // Converts a tail part into a ValidTarget if it is in vision range
    fn to_valid_target(&self, id: usize, target: &WormBody) -> Option<ValidTarget> {
        // check if the target is alive
        matches!(self.behaviors[id], WormBehavior::Alive(_))
            .then(|| {
                // check if the target tail is in vision range
                in_range(
                    self.details.origin,
                    self.details.chosen_destination,
                    *target.tail(),
                    &self.details.stats,
                )
                // return the Validtarget if the case
                .then(|| ValidTarget::from(self.details.origin, id, *target.tail()))
            })
            .flatten()
    }
}

impl Mover for ChasingWormMover<'_> {
    fn origin(&self) -> WormPart {
        self.details.origin
    }

    fn details(&self) -> &MovementDetails {
        self.details
    }

    /// Search for the closest worm tail in the visible range
    /// Return the index of the target worm in the table (if any) and its tail position
    /// (or a randon one if no target found)
    fn select_target(&self) -> (Option<usize>, Point) {
        match self
            .bodies
            .par_iter()
            .enumerate()
            // Filter the worms alive and in range, mapping their tail as a ValidTarget
            .filter_map(|(target_id, target)| self.to_valid_target(target_id, target))
            // choose the closest one
            .min_by(|lhs, rhs| lhs.distance.total_cmp(&rhs.distance))
        {
            Some(chosen_target) => (Some(chosen_target.target_id), chosen_target.target),
            // No valid target found, returns the destination according to the movement details
            None => (None, self.details.choose_destination()),
        }
    }

    fn collides(&self, part: WormPart, distance: f32) -> bool {
        self.bodies.par_iter().enumerate().any(|(pos, body)| {
            // Skip the tail of alive worms as they ar valid targets
            matches!(self.behaviors[pos], WormBehavior::Alive(_))
                .then_some(body.iter().take(body.size() - 1))
                .unwrap_or(body.iter().take(body.size()))
                // check for collision with all parts
                .any(|point| point.distance_to(part) < distance - 0.1)
        }) 
        // check for collision with rewards
        || self
            .rewards
            .par_iter()
            .any(|point| point.distance_to(part) < distance - 0.1)
    }
}

/// Checks if a given destination is in range of worm
/// (according to its head, direction and stats)
pub fn in_range(origin: Point, destination: Point, target: Point, stats: &WormStats) -> bool {
    origin
        .direction_to(destination)
        .connect(origin, target, stats.vision_range)
        && origin.distance_to(target) < stats.vision_distance
}

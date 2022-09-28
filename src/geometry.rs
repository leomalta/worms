use lazy_static::lazy_static;
use radians::{self, Radians};
use rand::{Rng, RngCore};
use std::fmt;
use std::{
    f32::consts::PI,
    ops::{Add, Sub},
};

pub type Angle = radians::Angle<f32, Radians>;

/// Number of possible movement directions (North, South, etc)
const N_DIRECTIONS: usize = 32;

lazy_static! {
    /// The arc covered by a direction (eg: 4 directions = 90°)
    static ref ARC_RANGE: Angle = Angle::new(2. * PI / N_DIRECTIONS as f32);
}

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Add<Point> for Point {
    type Output = Self;
    fn add(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Point> for Point {
    type Output = Self;
    fn sub(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.2}, {:.2})", self.x, self.y)
    }
}

impl Point {
    pub fn rand(xlimit: usize, ylimit: usize) -> Self {
        let rng = &mut rand::thread_rng();
        Self {
            x: rng.gen_range(0..=xlimit) as f32,
            y: rng.gen_range(0..=ylimit) as f32,
        }
    }

    pub fn unit() -> Self {
        Self { x: 1., y: 0. }
    }

    pub fn direction_to(self, other: &Point) -> Direction {
        Direction::from_radians(self.angle(other))
    }

    pub fn distance_to(&self, other: &Self) -> f32 {
        let diff = self.sub(*other);
        f32::hypot(diff.x, diff.y)
    }

    // Create a copy of the point at a given direction and distance
    pub fn copy(&self, direction: Direction, distance: f32) -> Self {
        self.add(direction.point().scale(distance))
    }

    pub fn angle(&self, other: &Self) -> Angle {
        let diff = other.sub(*self);
        Angle::new(f32::atan2(diff.y, diff.x))
    }

    pub fn scale(&self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }

    pub fn rotate(&self, angle: Angle) -> Self {
        Self {
            x: angle.cos() * self.x - angle.sin() * self.y,
            y: angle.sin() * self.x + angle.cos() * self.y,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct Direction {
    value: i16,
}

impl Add<Direction> for Direction {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dir({}, {})", self.value, self.to_radians().deg())
    }
}

impl Direction {
    pub fn rand() -> Self {
        Self {
            value: (rand::random::<u32>() % N_DIRECTIONS as u32) as i16,
        }
    }

    pub fn new(value: i16) -> Self {
        Self { value }
    }

    pub fn from_radians(angle: Angle) -> Self {
        Self {
            value: ((angle + (*ARC_RANGE / 2.)).val() / ARC_RANGE.val()).floor() as i16,
        }
    }

    /// Checks if the destination is at this direction from the origin, with a range tolerance
    /// i.e the direction 'connects' the origin to the destination
    pub fn connect(&self, origin: &Point, destination: &Point, range: Angle) -> bool {
        (self.to_radians().wrap() - origin.angle(destination)).mag() <= range / 2.
    }

    pub fn point(&self) -> Point {
        Point::unit().rotate(self.to_radians())
    }

    pub fn opposite(&self) -> Self {
        let dimensions = N_DIRECTIONS as i16;
        Self {
            value: ((dimensions / 2) + (self.value % dimensions)) % dimensions,
        }
    }

    pub fn to_radians(&self) -> Angle {
        *ARC_RANGE * self.value as f32
    }
}

/// Struct to change the direction following a specific order
/// +0, +1, -1, +2, -2, etc or +0, -1, +1, -2, +2,
pub struct Rotator {
    direction: Direction,
    times: usize,
    rotation: Rotation,
}

impl Rotator {
    pub fn new(direction: Direction) -> Self {
        let rng = &mut rand::thread_rng();
        let rotation = if rng.next_u64() % 2 == 0 {
            Rotation::Clockwise
        } else {
            Rotation::CounterClockwise
        };
        Self {
            direction,
            times: 0,
            rotation,
        }
    }
}

impl Iterator for Rotator {
    type Item = Direction;

    fn next(&mut self) -> Option<Direction> {
        if self.times == N_DIRECTIONS {
            return None;
        }
        self.times += 1;
        Some(self.direction + rotate(self.times as i16 - 1, self.rotation))
    }
}

#[derive(Clone, Copy)]
enum Rotation {
    Clockwise,
    CounterClockwise,
}

fn rotate(iteration: i16, rotation: Rotation) -> Direction {
    let value = match rotation {
        Rotation::Clockwise => {
            -(1 + ((iteration - 1) as f32 / 2.).floor() as i16)
                + 2 * (1 + ((iteration - 1) as f32 / 2.).floor() as i16) * ((iteration - 1) % 2)
        }
        Rotation::CounterClockwise => {
            (1 + ((iteration - 1) as f32 / 2.).floor() as i16)
                - 2 * (1 + ((iteration - 1) as f32 / 2.).floor() as i16) * ((iteration - 1) % 2)
        }
    };
    Direction::new(value)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use radians::{Angle, Degrees};

    use crate::geometry::{rotate, Direction, Point, Rotation, Rotator};

    #[test]
    fn connect() {
        let p1 = Point::unit();
        let p2 = Point { x: 0., y: 1. };
        let direction = Direction::from_radians(Angle::new(0.));
        let range: Angle<f32, Degrees> = Angle::new(90.);
        assert!(!direction.connect(&p1, &p2, range.rad()));

        let direction = Direction::from_radians(Angle::new(PI));
        assert!(direction.connect(&p1, &p2, range.rad()));

        let direction = Direction::from_radians(Angle::new(PI / 2.));
        assert!(direction.connect(&p1, &p2, range.rad()));

        let direction = Direction::from_radians(Angle::new(-PI / 2.));
        assert!(!direction.connect(&p1, &p2, range.rad()));

        let direction = Direction::from_radians(Angle::new(PI / 4.));
        assert!(!direction.connect(&p1, &p2, range.rad()));

        let direction = Direction::from_radians(Angle::new(3. * PI / 4.));
        assert!(direction.connect(&p1, &p2, range.rad()));

        let angle: Angle<f32, Degrees> = Angle::new(270.);
        let direction = Direction::from_radians(angle.rad());
        assert!(!direction.connect(&p1, &p2, range.rad()));
    }

    #[test]
    fn angle() {
        let p1 = Point::unit();
        let p2 = Point { x: 0., y: 1. };

        let angle = p1.angle(&p2);
        assert_eq!(angle.deg().val(), 135.);
        let angle = p2.angle(&p1);
        assert_eq!(angle.deg().val(), -45.);

        let direction = p1.direction_to(&p2);
        assert_eq!(direction.value, 3);
        let direction = p2.direction_to(&p1);
        assert_eq!(direction.value, -1);
    }

    #[test]
    fn from_radians() {
        let degrees: Angle<f32, Degrees> = Angle::new(-22.5);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, 0);

        let degrees: Angle<f32, Degrees> = Angle::new(22.4);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, 0);

        let degrees: Angle<f32, Degrees> = Angle::new(-67.5);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, -1);

        let degrees: Angle<f32, Degrees> = Angle::new(-22.6);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, -1);

        let degrees: Angle<f32, Degrees> = Angle::new(22.5);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, 1);

        let degrees: Angle<f32, Degrees> = Angle::new(67.4);
        let dir = Direction::from_radians(degrees.rad());
        assert_eq!(dir.value, 1);
    }

    #[test]
    fn shifts() {
        assert_eq!(rotate(0, Rotation::Clockwise), Direction::new(0));

        let direction = Direction::new(0);
        let mut rotator = Rotator {
            direction,
            times: 0,
            rotation: Rotation::Clockwise,
        };
        assert_eq!(rotator.next(), Some(Direction::new(0)));
        assert_eq!(rotator.next(), Some(Direction::new(-1)));
        assert_eq!(rotator.next(), Some(Direction::new(1)));
        assert_eq!(rotator.next(), Some(Direction::new(-2)));
        assert_eq!(rotator.next(), Some(Direction::new(2)));
        assert_eq!(rotator.next(), Some(Direction::new(-3)));
        assert_eq!(rotator.next(), Some(Direction::new(3)));
        assert_eq!(rotator.next(), Some(Direction::new(-4)));
        assert_eq!(rotator.next(), None);
    }
}

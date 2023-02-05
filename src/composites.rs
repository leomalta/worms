use crate::geometry::{Angle, Direction, Point};
use std::f32::consts::PI;
use std::fmt;

const MAX_SIZE: usize = 32;
pub type WormPart = Point;
pub type Reward = Point;

#[derive(Clone, Copy, Debug)]
pub enum WormBehavior {
    Alive(usize),
    Dead(usize),
    Chasing,
    Removed,
}

/// Fixed space allocated for all parts
type BodyContainer = [WormPart; MAX_SIZE];

/// Struct to hold all the parts of a worm (emulates a deque)
pub struct WormBody {
    pub target: Point,
    parts: BodyContainer,
    start: usize,
    size: usize,
}

impl Default for WormBody {
    fn default() -> Self {
        Self {
            target: Point::default(),
            parts: [Point::default(); MAX_SIZE],
            start: 0,
            size: 0,
        }
    }
}

impl WormBody {
    pub fn new(size: usize, head: WormPart, direction: Direction, part_size: f32) -> Self {
        // Allocate the array for the body parts
        let mut parts = [head; MAX_SIZE];
        // create the desired number of the body parts copying to the allocated array
        let start = size - 1;
        for i in (1..=start).rev() {
            parts[i - 1] = parts[i].copy(direction, part_size * 2.)
        }
        Self {
            target: head,
            parts,
            start,
            size,
        }
    }

    pub fn rand(size: usize, part_size: f32, xlimit: usize, ylimit: usize) -> Self {
        Self::new(
            size,
            WormPart::rand(xlimit, ylimit),
            Direction::rand(),
            part_size,
        )
    }

    pub fn head(&self) -> &WormPart {
        &self.parts[self.start]
    }

    pub fn tail(&self) -> &WormPart {
        &self.parts[(MAX_SIZE + self.start - self.size + 1) % MAX_SIZE]
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn full(&self) -> bool {
        self.size == MAX_SIZE
    }

    pub fn set_size(&mut self, size: usize) {
        self.size = size;
    }

    pub fn available_space(&self) -> usize {
        MAX_SIZE - self.size
    }

    pub fn shrink(&mut self, size: usize) {
        self.start = (MAX_SIZE + self.start - size) % MAX_SIZE;
        self.size -= size;
    }

    pub fn shift(&mut self, point: Point) {
        for i in 0..self.size {
            self.parts[(MAX_SIZE + self.start - i) % MAX_SIZE] =
                self.parts[(MAX_SIZE + self.start - i) % MAX_SIZE] + point;
        }
    }

    pub fn roll(&mut self, part: WormPart, target: Point) {
        self.start = (self.start + 1) % MAX_SIZE;
        self.parts[self.start] = part;
        self.target = target;
    }

    pub fn grow(&mut self, part: WormPart) {
        self.roll(part, part);
        self.size = MAX_SIZE.min(self.size + 1);
    }

    pub fn iter(&self) -> WormBodyIterator {
        WormBodyIterator {
            body: self,
            counter: 0,
        }
    }
}

impl fmt::Display for WormBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[ {}]",
            self.iter()
                .map(|part| part.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

pub struct WormBodyIterator<'a> {
    body: &'a WormBody,
    counter: usize,
}

impl<'a> Iterator for WormBodyIterator<'a> {
    type Item = &'a Point;

    fn next(&mut self) -> Option<Self::Item> {
        if self.counter < self.body.size {
            self.counter += 1;
            let pos = (1 + MAX_SIZE + self.body.start - self.counter) % MAX_SIZE;
            return Some(&self.body.parts[pos]);
        }
        None
    }
}

impl<'a> DoubleEndedIterator for WormBodyIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.counter < self.body.size {
            self.counter += 1;
            let pos = (MAX_SIZE + self.body.start + self.counter - self.body.size) % MAX_SIZE;
            return Some(&self.body.parts[pos]);
        }
        None
    }
}

#[derive(Clone, Copy)]
pub struct WormStats {
    pub vision_range: Angle,
    pub vision_distance: f32,
}

impl Default for WormStats {
    fn default() -> Self {
        Self {
            vision_range: Angle::new(5. * PI / 4.),
            vision_distance: 300.,
        }
    }
}

#[cfg(test)]
mod tests {
    use radians::{Angle, Degrees};

    use crate::geometry::{Direction, Point};

    use super::WormBody;

    #[test]
    fn bodies() {
        let direction = Direction::from_radians(Angle::new(0.));
        let mut worm = WormBody::new(4, Point::default(), direction, 5.0);
        let display = worm.to_string();
        assert_eq!(
            display,
            "[ (0.00, 0.00) (10.00, 0.00) (20.00, 0.00) (30.00, 0.00) ]".to_owned()
        );

        let mut angle: Angle<f32, Degrees> = Angle::new(0.);
        let mut new_head = *worm.head();
        for _ in 0..1000 {
            let direction = Direction::from_radians(angle.rad()).opposite();
            angle += Angle::new(90.);
            new_head = new_head.copy(direction, 10.);
            worm.roll(new_head, new_head);
        }
        let display = worm.to_string();
        assert_eq!(
            display,
            "[ (-0.00, 0.00) (-0.00, -10.00) (-10.00, -10.00) (-10.00, 0.00) ]".to_owned()
        );

        let direction = Direction::from_radians(angle.rad());
        for _ in 0..5 {
            new_head = new_head.copy(direction, 10.);
            worm.grow(new_head);
        }
        let display = worm.to_string();
        assert_eq!(
            display,
            "[ (50.00, -0.00) (40.00, -0.00) (30.00, -0.00) (20.00, -0.00) \
            (10.00, -0.00) (-0.00, 0.00) (-0.00, -10.00) (-10.00, -10.00) (-10.00, 0.00) ]"
                .to_owned()
        );
    }

    #[test]
    fn fill() {
        let direction = Direction::from_radians(Angle::new(0.));
        let worm1 = WormBody::new(4, Point::default(), direction, 5.0);
        let mut worm2 = WormBody::default();
        let display = worm1.to_string();
        assert_eq!(
            display,
            "[ (0.00, 0.00) (10.00, 0.00) (20.00, 0.00) (30.00, 0.00) ]".to_owned()
        );

        let parts = worm1.iter().rev().take(2).cloned().collect::<Vec<_>>();
        parts.into_iter().for_each(|part| worm2.grow(part));
        let display = worm2.to_string();
        assert_eq!(display, "[ (20.00, 0.00) (30.00, 0.00) ]".to_owned());
    }
}

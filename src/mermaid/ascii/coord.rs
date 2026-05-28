#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Coord {
    pub x: i32,
    pub y: i32,
}

impl Coord {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
    Middle,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::UpperLeft => Self::LowerRight,
            Self::UpperRight => Self::LowerLeft,
            Self::LowerLeft => Self::UpperRight,
            Self::LowerRight => Self::UpperLeft,
            Self::Middle => Self::Middle,
        }
    }

    pub fn from_to(from: Coord, to: Coord) -> Self {
        if from.x == to.x {
            if from.y < to.y {
                Self::Down
            } else if from.y > to.y {
                Self::Up
            } else {
                Self::Middle
            }
        } else if from.y == to.y {
            if from.x < to.x {
                Self::Right
            } else {
                Self::Left
            }
        } else if from.x < to.x {
            if from.y < to.y {
                Self::LowerRight
            } else {
                Self::UpperRight
            }
        } else if from.y < to.y {
            Self::LowerLeft
        } else {
            Self::UpperLeft
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_to_right() {
        assert_eq!(
            Direction::from_to(Coord::new(0, 0), Coord::new(5, 0)),
            Direction::Right
        );
    }
    #[test]
    fn from_to_left() {
        assert_eq!(
            Direction::from_to(Coord::new(5, 0), Coord::new(0, 0)),
            Direction::Left
        );
    }
    #[test]
    fn from_to_down() {
        assert_eq!(
            Direction::from_to(Coord::new(0, 0), Coord::new(0, 5)),
            Direction::Down
        );
    }
    #[test]
    fn from_to_up() {
        assert_eq!(
            Direction::from_to(Coord::new(0, 5), Coord::new(0, 0)),
            Direction::Up
        );
    }
    #[test]
    fn from_to_middle_when_equal() {
        assert_eq!(
            Direction::from_to(Coord::new(2, 2), Coord::new(2, 2)),
            Direction::Middle
        );
    }
    #[test]
    fn from_to_upper_right() {
        assert_eq!(
            Direction::from_to(Coord::new(0, 5), Coord::new(5, 0)),
            Direction::UpperRight
        );
    }
    #[test]
    fn from_to_lower_right() {
        assert_eq!(
            Direction::from_to(Coord::new(0, 0), Coord::new(5, 5)),
            Direction::LowerRight
        );
    }
    #[test]
    fn from_to_upper_left() {
        assert_eq!(
            Direction::from_to(Coord::new(5, 5), Coord::new(0, 0)),
            Direction::UpperLeft
        );
    }
    #[test]
    fn from_to_lower_left() {
        assert_eq!(
            Direction::from_to(Coord::new(5, 0), Coord::new(0, 5)),
            Direction::LowerLeft
        );
    }
    #[test]
    fn opposites() {
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::Left.opposite(), Direction::Right);
        assert_eq!(Direction::UpperRight.opposite(), Direction::LowerLeft);
        assert_eq!(Direction::Middle.opposite(), Direction::Middle);
    }
}

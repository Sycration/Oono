use std::{cmp::Ordering, fmt::Display, mem::discriminant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]

pub enum Color {
    Red,
    Green,
    Yellow,
    Blue,
    None,
}
impl Color {
    fn number(self) -> u8 {
        match self {
            Color::Red => 0,
            Color::Green => 1,
            Color::Yellow => 2,
            Color::Blue => 3,
            Color::None => 4,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Color::Red => "Red",
                Color::Green => "Green",
                Color::Yellow => "Yellow",
                Color::Blue => "Blue",
                Color::None => "",
            }
        )
    }
}

impl PartialOrd for Color {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.number().partial_cmp(&other.number())
    }
}

impl Ord for Color {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Card {
    Number(u8, Color),
    PlusTwo(Color),
    Reverse(Color),
    Skip(Color),
    Wild(Color),
    PlusFour(Color),
}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        //if discriminant(self) == discriminant(&Card::PlusFour(Color::None)) {
        //    return Some(Ordering::Greater);
        //}
        if self.color() == other.color() {
            if self.number() == other.number() {
                Some(Ordering::Equal)
            } else {
                self.number().partial_cmp(&other.number())
            }
        } else {
            self.color().partial_cmp(&other.color())
        }
    }
}

impl Card {
    #[must_use]
    pub fn color(&self) -> Color {
        match self {
            Card::Number(_, c)
            | Card::PlusTwo(c)
            | Card::Reverse(c)
            | Card::PlusFour(c)
            | Card::Skip(c)
            | Card::Wild(c) => *c,
        }
    }

    fn number(self) -> u8 {
        match self {
            Card::Number(n, _) => n,
            Card::PlusTwo(_) => 10,
            Card::Reverse(_) => 11,
            Card::Skip(_) => 12,
            Card::Wild(_) => 13,
            Card::PlusFour(_) => 14,
        }
    }
    #[must_use]
    pub fn is_valid_on(&self, other: &Self) -> bool {
        self.color() == other.color()
            || self.number() == other.number()
            || discriminant(self) == discriminant(&Card::PlusFour(Color::None))
            || discriminant(self) == discriminant(&Card::Wild(Color::None))
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Card::Number(n, _) => n.to_string(),
                Card::PlusTwo(_) => "Plus Two".to_string(),
                Card::Reverse(_) => "Reverse".to_string(),
                Card::PlusFour(_) => "Plus Four".to_string(),
                Card::Skip(_) => "Skip".to_string(),
                Card::Wild(_) => "Wild".to_string(),
            }
        )
    }
}

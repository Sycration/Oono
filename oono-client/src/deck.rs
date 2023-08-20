use crate::card::{Card, Color};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]

pub struct Deck(pub Vec<Card>);
impl Deck {
    #[must_use]
    pub fn new_empty() -> Self {
        Self(vec![])
    }

    #[must_use]
    pub fn new_full() -> Self {
        let mut arr = vec![];
        for c in [Color::Red, Color::Green, Color::Blue, Color::Yellow] {
            for n in 0..=9 {
                arr.push(Card::Number(n, c));
                arr.push(Card::Number(n, c));
            }
            arr.push(Card::Skip(c));
            arr.push(Card::Reverse(c));
            arr.push(Card::PlusTwo(c));
        }
        for _ in 0..4 {
            arr.push(Card::PlusFour(Color::None));
            arr.push(Card::Wild(Color::None));
        }

        let mut rng = thread_rng();
        arr.shuffle(&mut rng);
        Self(arr)
    }
}

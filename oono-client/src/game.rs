use std::{collections::HashMap, mem::discriminant};

use rand::prelude::*;
use uuid::Uuid;

use crate::{
    card::{Card, Color},
    deck::Deck,
};
#[derive(Debug, PartialEq, Eq)]
pub struct Game {
    pub pot: Deck,
    pub discard: Deck,

    pub creator_token: Uuid,
    pub reversed: bool,
    pub players: HashMap<Uuid, Player>,
    pub started: bool,
    pub whos_turn: usize,
}

impl Game {
    pub fn increment_turn(&mut self) {
        self.whos_turn = match self.reversed {
            true => ((self.whos_turn + self.players.len()) - 1) % self.players.len(),
            false => (self.whos_turn + 1) % self.players.len(),
        }
    }

    pub fn pop_pot(&mut self) -> Card {
        let last_card = *self.discard.0.last().unwrap();

        if self.pot.0.is_empty() {
            if self.discard.0.len() <= 1 {
                self.discard.0.append(&mut Deck::new_full().0);
            }
            self.pot.0.append(&mut self.discard.0);
            self.pot.0.shuffle(&mut thread_rng());
            self.pot.0.iter_mut().for_each(|c| {
                if discriminant(c) == discriminant(&Card::Wild(Color::None)) {
                    *c = Card::Wild(Color::None);
                } else if discriminant(c) == discriminant(&Card::PlusFour(Color::None)) {
                    *c = Card::PlusFour(Color::None);
                }
            });
            self.discard.0.push(last_card);
        }
        self.pot.0.pop().unwrap()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]

pub struct Player {
    pub name: String,
    pub order_num: usize,

    pub hand: Deck,
}

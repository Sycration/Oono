use dashmap::DashMap;
use oono::{
    card::{Card, Color},
    deck::Deck,
    event::{Error, OpaquePlayer, Server},
    game::{Game, Player},
    *,
};
use rand::{thread_rng, Rng};
use rocket::{
    serde::json::Json,
    tokio::{self, time::sleep},
    Config, State,
};

use std::{collections::HashMap, mem::discriminant, net::Ipv4Addr, sync::Arc};
use uuid::Uuid;

#[macro_use]
extern crate rocket;

#[get("/CreateGame")]
fn create_game(
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, oono::event::Error>> {
    let mut pot = Deck::new_full();
    let mut discard = Deck::new_empty();
    loop {
        discard.0.push(pot.0.pop().unwrap());
        if discriminant(discard.0.last().unwrap()) == discriminant(&Card::Wild(Color::None))
            || discriminant(discard.0.last().unwrap()) == discriminant(&Card::PlusFour(Color::None))
        {
            continue;
        } else {
            break;
        }
    }
    let game = Game {
        pot,
        discard,
        creator_token: Uuid::new_v4(),
        reversed: false,
        players: HashMap::new(),
        started: false,
        whos_turn: 0,
    };
    let id = Uuid::new_v4();
    let token = game.creator_token;
    games.insert(id, game);
    Json(Ok(Server::GameCreated {
        game_id_ret: id,
        gm_token_ret: token,
    }))
}

#[get("/JoinGame/<game_id>/<name>")]
fn join_game(
    game_id: String,
    name: String,
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, Error>> {
    let game_id = match Uuid::parse_str(&game_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: game_id,
                error: e.to_string(),
            }));
        }
    };

    let mut game = match games.get_mut(&game_id) {
        Some(game) => game,
        None => {
            return Json(Err(Error::GameDoesNotExist { game_id }));
        }
    };

    let order_num = game.players.len();
    let mut hand = vec![];
    for _ in 0..7 {
        if let Some(c) = game.pot.0.pop() {
            hand.push(c);
        } else {
            let mut new_pot = Deck::new_full().0;
            game.pot.0.append(&mut new_pot);
            //this cannot fail as we just added a hundred new cards
            hand.push(game.pot.0.pop().unwrap());
        }
    }

    hand.sort();

    let player = Player {
        name,
        order_num,
        hand: Deck(hand),
    };
    let player_id = Uuid::new_v4();
    game.players.insert(player_id, player);
    Json(Ok(Server::GameJoined {
        player_id_ret: player_id,
        order_num_ret: order_num,
        game_id_ret: game_id,
    }))
}

#[get("/StartGame/<game_id>/<gm_token>")]
fn start_game(
    game_id: String,
    gm_token: String,
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, Error>> {
    let game_id = match Uuid::parse_str(&game_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: game_id,
                error: e.to_string(),
            }));
        }
    };
    let gm_token = match Uuid::parse_str(&gm_token) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: gm_token,
                error: e.to_string(),
            }));
        }
    };

    let mut game = match games.get_mut(&game_id) {
        Some(game) => game,
        None => {
            return Json(Err(Error::GameDoesNotExist { game_id }));
        }
    };

    if gm_token == game.creator_token {
        let whos_first = thread_rng().gen_range(0..game.players.len());
        game.whos_turn = whos_first;
        game.started = true;
        Json(Ok(Server::GameStarted))
    } else {
        Json(Err(Error::InvalidGMToken {
            bad_token: gm_token,
        }))
    }
}

#[get("/RequestUpdate/<game_id>/<player_id>")]
fn request_update(
    game_id: String,
    player_id: String,
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, Error>> {
    let game_id = match Uuid::parse_str(&game_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: game_id,
                error: e.to_string(),
            }));
        }
    };
    let player_id = match Uuid::parse_str(&player_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: player_id,
                error: e.to_string(),
            }));
        }
    };

    let game = match games.get_mut(&game_id) {
        Some(game) => game,
        None => {
            return Json(Err(Error::GameDoesNotExist { game_id }));
        }
    };

    for player in game.players.values() {
        if player.hand.0.is_empty() {
            return Json(Ok(Server::PlayerWon {
                order_num: player.order_num,
            }));
        }
    }

    let player = if let Some(p) = game.players.get(&player_id) {
        p
    } else {
        return Json(Err(Error::PlayerDoesNotExist { player_id }));
    };

    Json(Ok(Server::UpdateResponse {
        hand_ret: player.hand.clone(),
        discard_ret: *game.discard.0.last().unwrap(),
        reversed_ret: game.reversed,
        players_ret: game
            .players
            .iter()
            .map(
                |(
                    _,
                    Player {
                        name,
                        order_num,
                        hand,
                    },
                )| {
                    OpaquePlayer {
                        order_num: *order_num,
                        hand_size: hand.0.len(),
                        name: name.to_string(),
                    }
                },
            )
            .collect(),
        whose_turn_ret: game.whos_turn,
        playing_ret: game.started,
        pot_size_ret: game.pot.0.len(),
    }))
}

#[get("/PlaceCard/<game_id>/<player_id>/<index>/<color>")]
fn place_card(
    game_id: String,
    player_id: String,
    index: usize,
    color: String,
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, Error>> {
    let games = games.inner().clone();
    let games_for_dtor = games.clone();
    let game_id = match Uuid::parse_str(&game_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: game_id,
                error: e.to_string(),
            }));
        }
    };
    let player_id = match Uuid::parse_str(&player_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: player_id,
                error: e.to_string(),
            }));
        }
    };

    let mut game = match games.get_mut(&game_id) {
        Some(game) => game,
        None => {
            return Json(Err(Error::GameDoesNotExist { game_id }));
        }
    };
    let whos_turn = game.whos_turn;
    let discard = *game.discard.0.last().unwrap();

    let player = if let Some(p) = game.players.get_mut(&player_id) {
        p
    } else {
        return Json(Err(Error::PlayerDoesNotExist { player_id }));
    };

    let mut card = {
        if index > player.hand.0.len() {
            return Json(Err(Error::CardOutOfRange { index }));
        } else {
            let c = player.hand.0.remove(index);
            player.hand.0.sort();
            c
        }
    };

    if !card.is_valid_on(&discard) || whos_turn != player.order_num {
        return Json(Err(Error::IllegalMove));
    }

    if player.hand.0.is_empty() {
        let order_num = player.order_num;
        tokio::spawn(async move {
            sleep(std::time::Duration::from_secs(60)).await;
            games_for_dtor.remove(&game_id);
        });
        return Json(Ok(Server::PlayerWon { order_num }));
    }

    match card {
        card::Card::Number(_, _) => {}
        card::Card::PlusTwo(_) => {
            let card1 = game.pop_pot();
            let card2 = game.pop_pot();
            let id = if let Some((id, _)) = game.players.iter().find(|p| {
                p.1.order_num
                    == match game.reversed {
                        true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
                        false => (game.whos_turn + 1) % game.players.len(),
                    }
            }) {
                *id
            } else {
                return Json(Err(Error::IllegalMove));
            };
            let next_player = if let Some(p) = game.players.get_mut(&id) {
                p
            } else {
                return Json(Err(Error::IllegalMove));
            };
            next_player.hand.0.push(card1);
            next_player.hand.0.push(card2);
            next_player.hand.0.sort();
            game.whos_turn = match game.reversed {
                true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
                false => (game.whos_turn + 1) % game.players.len(),
            };
        }
        card::Card::Reverse(_) => {
            game.reversed = !game.reversed;
        }
        card::Card::Skip(_) => {
            game.whos_turn = match game.reversed {
                true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
                false => (game.whos_turn + 1) % game.players.len(),
            };
        }
        card::Card::Wild(_) => {
            card = Card::Wild(match color.as_str() {
                "Red" => Color::Red,
                "Green" => Color::Green,
                "Yellow" => Color::Yellow,
                "Blue" => Color::Blue,
                _ => {
                    return Json(Err(Error::IllegalMove));
                }
            });
        }
        card::Card::PlusFour(_) => {
            card = Card::PlusFour(match color.as_str() {
                "Red" => Color::Red,
                "Green" => Color::Green,
                "Yellow" => Color::Yellow,
                "Blue" => Color::Blue,
                _ => {
                    return Json(Err(Error::IllegalMove));
                }
            });
            let card1 = game.pop_pot();
            let card2 = game.pop_pot();
            let card3 = game.pop_pot();
            let card4 = game.pop_pot();
            let id = if let Some((id, _)) = game.players.iter().find(|p| {
                p.1.order_num
                    == match game.reversed {
                        true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
                        false => (game.whos_turn + 1) % game.players.len(),
                    }
            }) {
                *id
            } else {
                return Json(Err(Error::IllegalMove));
            };
            let next_player = if let Some(p) = game.players.get_mut(&id) {
                p
            } else {
                return Json(Err(Error::IllegalMove));
            };
            next_player.hand.0.push(card1);
            next_player.hand.0.push(card2);
            next_player.hand.0.push(card3);
            next_player.hand.0.push(card4);
            next_player.hand.0.sort();
            game.whos_turn = match game.reversed {
                true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
                false => (game.whos_turn + 1) % game.players.len(),
            };
        }
    }

    game.discard.0.push(card);

    game.whos_turn = match game.reversed {
        true => ((game.whos_turn + game.players.len()) - 1) % game.players.len(),
        false => (game.whos_turn + 1) % game.players.len(),
    };

    Json(Ok(Server::CardPlaced))
}

#[get("/DrawCard/<game_id>/<player_id>")]
fn draw_card(
    game_id: String,
    player_id: String,
    games: &State<Arc<DashMap<Uuid, Game>>>,
) -> Json<Result<Server, Error>> {
    let game_id = match Uuid::parse_str(&game_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: game_id,
                error: e.to_string(),
            }));
        }
    };
    let player_id = match Uuid::parse_str(&player_id) {
        Ok(id) => id,
        Err(e) => {
            return Json(Err(Error::InvalidUuid {
                id: player_id,
                error: e.to_string(),
            }));
        }
    };

    let mut game = match games.get_mut(&game_id) {
        Some(game) => game,
        None => {
            return Json(Err(Error::GameDoesNotExist { game_id }));
        }
    };
    let card = game.pop_pot();
    let whos_turn = game.whos_turn;
    let player = if let Some(p) = game.players.get_mut(&player_id) {
        p
    } else {
        return Json(Err(Error::PlayerDoesNotExist { player_id }));
    };

    let hand = &mut player.hand.0;

    if whos_turn == player.order_num {
        hand.push(card);
        hand.sort();
    } else {
        return Json(Err(Error::IllegalMove));
    }

    Json(Ok(Server::CardDrawn))
}

#[launch]
fn rocket() -> _ {
    let config = Config {
        address: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        port: 8000,
        ..Default::default()
    };
    rocket::build()
        .configure(config)
        .manage(Arc::new(DashMap::<Uuid, Game>::new()))
        .mount(
            "/",
            routes![
                create_game,
                join_game,
                start_game,
                request_update,
                place_card,
                draw_card
            ],
        )
}

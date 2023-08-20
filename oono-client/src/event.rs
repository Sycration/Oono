use std::fmt::Display;
use std::thread;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::card::{Card, Color};
use crate::deck::Deck;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{Receiver, Sender};

use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;
#[derive(Serialize, Deserialize)]
pub enum Client {
    UpdateServer {
        url: String,
    },
    CreateGame,
    JoinGame {
        game_id: Uuid,
        name: String,
    },
    StartGame {
        game_id: Uuid,
        gm_token: Uuid,
    },
    RequestUpdate {
        game_id: Uuid,
        player_id: Uuid,
    },
    PlaceCard {
        game_id: Uuid,
        player_id: Uuid,
        index: usize,
        color: Option<Color>,
    },
    DrawCard {
        game_id: Uuid,
        player_id: Uuid,
    },
}

#[derive(Serialize, Deserialize, Clone)]

pub struct OpaquePlayer {
    pub order_num: usize,
    pub hand_size: usize,
    pub name: String,
}

#[derive(Serialize, Deserialize)]

pub enum Server {
    GameCreated {
        game_id_ret: Uuid,
        gm_token_ret: Uuid,
    },
    GameJoined {
        game_id_ret: Uuid,
        player_id_ret: Uuid,
        order_num_ret: usize,
    },
    GameStarted,
    UpdateResponse {
        playing_ret: bool,
        hand_ret: Deck,
        discard_ret: Card,
        reversed_ret: bool,
        players_ret: Vec<OpaquePlayer>,
        whose_turn_ret: usize,
        pot_size_ret: usize,
    },
    CardPlaced,
    CardDrawn,
    PlayerWon {
        order_num: usize,
    },
}
#[derive(Serialize, Deserialize)]

pub enum Error {
    CouldNotContactServer { url: String, error: String },
    MalformedResponse { error: String },
    InvalidUuid { id: String, error: String },
    GameDoesNotExist { game_id: Uuid },
    PlayerDoesNotExist { player_id: Uuid },
    InvalidGMToken { bad_token: Uuid },
    CardOutOfRange { index: usize },
    IllegalMove,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CouldNotContactServer { url, error } => write!(f, "Could not contact the server at {} . Check your internet connection and provide the admin with the following error message:\n{}",url, error),
            Error::InvalidUuid { id, error } => write!(
                f,
                "{} is not a valid UUID.\nThe error generated was:\n{}",
                id, error
            ),
            Error::GameDoesNotExist { game_id } => write!(f, "{} is not a valid game ID. Make sure you have the correct ID.", game_id),
            Error::MalformedResponse { error } => write!(f, "The program recieved a malformed response from the server:\n{} ", error),
            Error::PlayerDoesNotExist { player_id } => write!(f, "{} is not a valid player ID. Tell the admin about this.", player_id),
            Error::InvalidGMToken { bad_token } => write!(f, "{} is not the correct GM token. Stop cheating!", bad_token),
            Error::CardOutOfRange { index } => write!(f, "Card {} is out of range. Stop cheating!", index),
            Error::IllegalMove => write!(f, "That move is illegal. Stop cheating!"),
        }
    }
}

pub fn handle_events(
    client_evt_reciever: Receiver<Client>,
    server_evt_sender: Sender<Result<Server, Error>>,
    server_url: String,
) {
    thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut server_url = server_url.clone();
                let mut in_stream = ReceiverStream::new(client_evt_reciever);
                let client = reqwest::ClientBuilder::new()
                    .timeout(Duration::from_millis(450))
                    .build()
                    .unwrap();
                loop {
                    match in_stream.next().await {
                        Some(evt) => {
                            match client
                                .get(match evt {
                                    Client::UpdateServer { url } => {
                                        server_url = url;
                                        continue;
                                    }
                                    Client::CreateGame => {
                                        format!("{}/CreateGame", &server_url)
                                    }
                                    Client::JoinGame { game_id, name } => {
                                        format!("{}/JoinGame/{}/{}", &server_url, game_id, name)
                                    }
                                    Client::StartGame { game_id, gm_token } => {
                                        format!(
                                            "{}/StartGame/{}/{}",
                                            &server_url, game_id, gm_token
                                        )
                                    }
                                    Client::RequestUpdate { game_id, player_id } => format!(
                                        "{}/RequestUpdate/{}/{}",
                                        &server_url, game_id, player_id
                                    ),
                                    Client::PlaceCard {
                                        game_id,
                                        player_id,
                                        index,
                                        color,
                                    } => format!(
                                        "{}/PlaceCard/{}/{}/{}/{}",
                                        &server_url,
                                        game_id,
                                        player_id,
                                        index,
                                        if let Some(c) = color {
                                            c.to_string()
                                        } else {
                                            "None".to_string()
                                        }
                                    ),
                                    Client::DrawCard { game_id, player_id } => format!(
                                        "{}/DrawCard/{}/{}",
                                        &server_url, game_id, player_id
                                    ),
                                })
                                .send()
                                .await
                            {
                                Ok(r) => match r.json::<Result<Server, Error>>().await {
                                    Ok(r) => {
                                        server_evt_sender.send(r).await;
                                    }
                                    Err(e) => {
                                        server_evt_sender
                                            .send(Err(Error::MalformedResponse {
                                                error: e.to_string(),
                                            }))
                                            .await;
                                    }
                                },
                                Err(e) => {
                                    server_evt_sender
                                        .send(Err(Error::CouldNotContactServer {
                                            url: (&server_url).to_string(),
                                            error: e.to_string(),
                                        }))
                                        .await;
                                }
                            }
                        }
                        None => (),
                    }
                }
            });
    });
}

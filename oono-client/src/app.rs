use crate::event::OpaquePlayer;
use crate::{
    card::{Card, Color},
    deck::Deck,
    event::{handle_events, Client, Error, Server},
};
use egui::{Align, Button, Layout, RichText, ScrollArea, TextEdit, Visuals};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use uuid::Uuid;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct OonoApp {
    // Example stuff:
    #[serde(skip)]
    my_hand: Deck,
    #[serde(skip)]
    discard: Card,
    #[serde(skip)]
    choosing_wild: Option<usize>,
    #[serde(skip)]
    choosing_p4: bool,
    #[serde(skip)]
    tx: Sender<Client>,
    #[serde(skip)]
    rx: Receiver<Result<Server, Error>>,
    #[serde(skip)]
    game_id_string: String,
    #[serde(skip)]
    game_id: Option<Uuid>,
    #[serde(skip)]
    gm_token: Option<Uuid>,
    #[serde(skip)]
    player_id: Option<Uuid>,
    #[serde(skip)]
    order_num: Option<usize>,
    #[serde(skip)]
    player_name: String,
    #[serde(skip)]
    error_msg: Option<String>,
    #[serde(skip)]
    last_update: Instant,
    #[serde(skip)]
    players: Vec<OpaquePlayer>,
    #[serde(skip)]
    reversed: bool,
    #[serde(skip)]
    whose_turn: usize,
    #[serde(skip)]
    playing: bool,
    #[serde(skip)]
    pot_size: usize,
    #[serde(skip)]
    winner: Option<OpaquePlayer>,
    url: String,
}

impl Default for OonoApp {
    fn default() -> Self {
        let (client_evt_tx, client_evt_rx) = channel(25);
        let (server_evt_tx, server_evt_rx) = channel(25);
        handle_events(client_evt_rx, server_evt_tx, String::new());
        Self {
            // Example stuff:
            my_hand: Deck::new_empty(),
            discard: Card::Wild(Color::None),
            choosing_wild: None,
            choosing_p4: false,
            tx: client_evt_tx,
            rx: server_evt_rx,
            game_id: None,
            gm_token: None,
            player_id: None,
            order_num: None,
            player_name: String::new(),
            game_id_string: String::new(),
            error_msg: None,
            last_update: Instant::now(),
            players: Vec::new(),
            reversed: false,
            whose_turn: 0,
            playing: false,
            pot_size: 0,
            winner: None,
            url: "http://server.com:1234".to_string(),
        }
    }
}

impl OonoApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(Visuals::dark());
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for OonoApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self {
            my_hand,
            discard,
            choosing_wild,
            choosing_p4,
            tx,
            rx,
            game_id,
            gm_token,
            player_id,
            order_num,
            player_name,
            game_id_string,
            error_msg,
            last_update,
            players,
            reversed,
            whose_turn,
            playing,
            pot_size,
            winner,
            url,
        } = self;
        let now = Instant::now();
        if now.duration_since(*last_update) > Duration::from_millis(500) {
            if let (Some(game_id), Some(player_id)) = (*game_id, *player_id) {
                tx.try_send(Client::RequestUpdate { game_id, player_id });
                *last_update = now;
            }
        }
        if let Ok(evt) = rx.try_recv() {
            match evt {
                Ok(evt) => match evt {
                    Server::GameCreated {
                        game_id_ret,
                        gm_token_ret,
                    } => {
                        *gm_token = Some(gm_token_ret);
                        *game_id = Some(game_id_ret);
                        tx.try_send(Client::JoinGame {
                            game_id: game_id_ret,
                            name: player_name.to_string(),
                        });
                    }
                    Server::GameJoined {
                        player_id_ret,
                        order_num_ret,
                        game_id_ret,
                    } => {
                        *player_id = Some(player_id_ret);
                        *order_num = Some(order_num_ret);
                        *game_id = Some(game_id_ret);
                    }
                    Server::GameStarted => {
                        if let (Some(game_id), Some(player_id)) = (*game_id, *player_id) {
                            tx.try_send(Client::RequestUpdate { game_id, player_id });
                            *last_update = now;
                        }
                    }
                    Server::UpdateResponse {
                        hand_ret,
                        discard_ret,
                        reversed_ret,
                        players_ret,
                        whose_turn_ret,
                        playing_ret,
                        pot_size_ret,
                    } => {
                        *players = players_ret;
                        *my_hand = hand_ret;
                        *discard = discard_ret;
                        *reversed = reversed_ret;
                        *whose_turn = whose_turn_ret;
                        *playing = playing_ret;
                        *pot_size = pot_size_ret;
                    }
                    Server::CardPlaced => {
                        if let (Some(game_id), Some(player_id)) = (*game_id, *player_id) {
                            tx.try_send(Client::RequestUpdate { game_id, player_id });
                            *last_update = now;
                        }
                    }
                    Server::CardDrawn => {
                        if let (Some(game_id), Some(player_id)) = (*game_id, *player_id) {
                            tx.try_send(Client::RequestUpdate { game_id, player_id });
                            *last_update = now;
                        }
                    }
                    Server::PlayerWon { order_num } => {
                        if let Some(w) = players.iter().find(|p| p.order_num == order_num) {
                            let w = (*w).clone();
                            *winner = Some(w);
                        } else {
                            *error_msg = Some("A nonexistant player has won the game. Please contact the administrator.".to_owned());
                        }
                    }
                },
                Err(e) => *error_msg = Some(e.to_string()),
            }
        }

        if game_id.is_none() {
            egui::Window::new("Welcome to Oono :)")
                //.min_width(300.)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.heading("Create game");
                            if ui
                                .add_enabled(!player_name.is_empty(), Button::new("Create"))
                                .clicked()
                            {
                            tx.try_send(Client::UpdateServer { url: url.to_string() });

                                tx.try_send(Client::CreateGame );

                            }
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.heading("Join game");
                            ui.horizontal(|ui| {
                                ui.label("Game ID: ");
                                ui.text_edit_singleline(game_id_string);
                            });
                            if ui
                                .add_enabled(
                                    !player_name.is_empty() && !game_id_string.is_empty(),
                                    Button::new("Join"),
                                )
                                .clicked()
                            {
                                match Uuid::parse_str(game_id_string) {
                                    Ok(id) => {
                                         tx.try_send(Client::UpdateServer { url: url.to_string() });
                                        
                                        tx.try_send(Client::JoinGame {
                                            game_id: id,
                                            name: player_name.to_string(),
                                        });

                                        *error_msg = None;
                                    }
                                    Err(e) => {
                                        *error_msg =
                                            Some(format!("Error:  {}  is not a valid UUID.\nThe error generated was:\n{}", game_id_string, e));
                                    }
                                }
                            }

                        });
                    });
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Name: ");
                        ui.text_edit_singleline(player_name);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Server: ");
                        if ui.text_edit_singleline(url).changed() {
                            tx.try_send(Client::UpdateServer { url: url.to_string() });
                        };

                    });

                });
        }

        if let Some(id) = *game_id {
            if gm_token.is_some() {
                egui::Window::new("GM panel")
                    //.min_width(300.)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Game ID");
                        ui.add(TextEdit::singleline(&mut id.to_string()).code_editor());
                        if !*playing && ui.button("Start game").clicked() {
                            tx.try_send(Client::UpdateServer { url: url.to_string() });
                            if let (Some(game_id), Some(gm_token)) = (*game_id, *gm_token) {
                                ui.separator();
                                tx.try_send(Client::StartGame { game_id, gm_token });
                            }
                        }
                    });
            }
        }

        if let Some(w) = winner.clone() {
            egui::Window::new("Winner")
                //.min_width(300.)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading(format!("{} has won the game!", w.name));
                    ui.horizontal(|ui| {
                        if ui.button("New game").clicked() {
                            *my_hand = Deck::new_empty();
                            *discard = Card::Wild(Color::None);
                            *choosing_wild = None;
                            *choosing_p4 = false;
                            *game_id = None;
                            *gm_token = None;
                            *player_id = None;
                            *order_num = None;
                            *player_name = String::new();
                            *game_id_string = String::new();
                            *error_msg = None;
                            *last_update = Instant::now();
                            *players = Vec::new();
                            *reversed = false;
                            *whose_turn = 0;
                            *playing = false;
                            *pot_size = 0;
                            *winner = None;
                        }
                        if ui.button("quit").clicked() {
                            frame.quit();
                        }
                    });
                });
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading(format!("You are {}", *player_name));
            ui.heading("Opponents");
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| {
                ui.label(format!("whos turn: {}", *whose_turn));

                for player in &*players {
                    egui::containers::Frame::group(ctx.style().as_ref()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(&player.name);
                            //ui.label(format!("order: {}", player.order_num));

                            if *playing {
                                if player.hand_size == 1 {
                                    ui.label("UNO CARDS!");
                                } else {
                                    ui.label(format!("{} cards", player.hand_size));
                                }
                                if *whose_turn == player.order_num {
                                    ui.label(RichText::new("Playing now").strong());
                                }
                                if (!*reversed
                                    && *whose_turn
                                        == (player.order_num + players.len() - 1) % players.len())
                                    || (*reversed
                                        && *whose_turn
                                            == (player.order_num + players.len() + 1)
                                                % players.len())
                                {
                                    ui.label(RichText::new("Playing next").strong());
                                }
                            }
                        });
                    });
                }
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("By SVX");
                });
            });
        });

        if let Some(msg) = error_msg.clone() {
            egui::Window::new("Error")
                //.min_width(300.)
                .auto_sized()
                .show(ctx, |ui| {
                    ui.add(TextEdit::multiline(&mut msg.to_string()).code_editor());
                    if ui.button("    close    ").clicked() {
                        *error_msg = None;
                    }
                });
        }

        if let Some(index) = *choosing_wild {
            egui::Window::new("Select a color").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    //red
                    if ui
                        .add(
                            Button::new(
                                RichText::new("RED")
                                    .color(egui::color::Color32::from_rgb(50, 0, 0)),
                            )
                            .fill(egui::color::Color32::from_rgb(255, 0, 0)),
                        )
                        .clicked()
                    {
                        if let Some(game_id) = *game_id {
                            if let Some(player_id) = *player_id {
                                tx.try_send(Client::PlaceCard {
                                    game_id,
                                    player_id,
                                    index,
                                    color: Some(Color::Red),
                                });
                                *choosing_p4 = false;
                                *choosing_wild = None;
                            }
                        }
                    }

                    //green
                    if ui
                        .add(
                            Button::new(
                                RichText::new("GREEN")
                                    .color(egui::color::Color32::from_rgb(0, 50, 0)),
                            )
                            .fill(egui::color::Color32::from_rgb(0, 255, 0)),
                        )
                        .clicked()
                    {
                        if let Some(game_id) = *game_id {
                            if let Some(player_id) = *player_id {
                                tx.try_send(Client::PlaceCard {
                                    game_id,
                                    player_id,
                                    index,
                                    color: Some(Color::Green),
                                });
                                *choosing_p4 = false;
                                *choosing_wild = None;
                            }
                        }
                    }

                    //yellow
                    if ui
                        .add(
                            Button::new(
                                RichText::new("YELLOW")
                                    .color(egui::color::Color32::from_rgb(0, 50, 50)),
                            )
                            .fill(egui::color::Color32::from_rgb(255, 255, 0)),
                        )
                        .clicked()
                    {
                        if let Some(game_id) = *game_id {
                            if let Some(player_id) = *player_id {
                                tx.try_send(Client::PlaceCard {
                                    game_id,
                                    player_id,
                                    index,
                                    color: Some(Color::Yellow),
                                });
                                *choosing_p4 = false;
                                *choosing_wild = None;
                            }
                        }
                    }

                    //blue
                    if ui
                        .add(
                            Button::new(
                                RichText::new("BLUE")
                                    .color(egui::color::Color32::from_rgb(0, 0, 0)),
                            )
                            .fill(egui::color::Color32::from_rgb(0, 125, 255)),
                        )
                        .clicked()
                    {
                        if let Some(game_id) = *game_id {
                            if let Some(player_id) = *player_id {
                                tx.try_send(Client::PlaceCard {
                                    game_id,
                                    player_id,
                                    index,
                                    color: Some(Color::Blue),
                                });
                                *choosing_p4 = false;
                                *choosing_wild = None;
                            }
                        }
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.add_enabled_ui(
                    *playing
                        && if let Some(n) = *order_num {
                            n == *whose_turn
                        } else {
                            false
                        },
                    |ui| {
                        ui.label(RichText::new("My Hand").text_style(egui::TextStyle::Heading));
                        ScrollArea::horizontal().max_height(25.).show(ui, |ui| {
                            ui.with_layout(Layout::left_to_right(), |ui| {
                                for (idx, c) in my_hand.0.iter_mut().enumerate() {
                                    let (dim, bright) = match c.color() {
                                        Color::Red => (
                                            egui::color::Color32::from_rgb(50, 0, 0),
                                            egui::color::Color32::from_rgb(255, 0, 0),
                                        ),
                                        Color::Green => (
                                            egui::color::Color32::from_rgb(0, 50, 0),
                                            egui::color::Color32::from_rgb(0, 255, 0),
                                        ),
                                        Color::Yellow => (
                                            egui::color::Color32::from_rgb(0, 50, 50),
                                            egui::color::Color32::from_rgb(255, 255, 0),
                                        ),
                                        Color::Blue => (
                                            egui::color::Color32::from_rgb(0, 0, 0),
                                            egui::color::Color32::from_rgb(0, 125, 255),
                                        ),
                                        Color::None => (
                                            egui::color::Color32::from_rgb(227, 227, 227),
                                            egui::color::Color32::from_rgb(70, 70, 70),
                                        ),
                                    };
                                    let text =
                                        RichText::new(format!("{}\n{}", c, c.color())).color(dim);
                                    ui.vertical(|ui| {
                                        if !c.is_valid_on(discard) {
                                            ui.add_enabled(
                                                false,
                                                Button::new(text).wrap(false).fill(bright),
                                            );
                                        } else if ui
                                            .add(Button::new(text).wrap(false).fill(bright))
                                            .clicked()
                                        {
                                            if *c == Card::PlusFour(Color::None) {
                                                *choosing_wild = Some(idx);
                                                *choosing_p4 = true;
                                            } else if *c == Card::Wild(Color::None) {
                                                *choosing_wild = Some(idx);
                                                *choosing_p4 = false;
                                            } else if let (Some(game_id), Some(player_id)) =
                                                (*game_id, *player_id)
                                            {
                                                tx.try_send(Client::PlaceCard {
                                                    game_id,
                                                    player_id,
                                                    index: idx,
                                                    color: None,
                                                });
                                            }
                                        }
                                    });
                                }
                            });
                        });

                        if ui.button("Draw card").clicked() {
                            if let (Some(game_id), Some(player_id)) = (*game_id, *player_id) {
                                tx.try_send(Client::DrawCard { game_id, player_id });
                            }
                        }
                        ui.label(format!("{} cards remain in the pot.", *pot_size));

                        ui.separator();
                        ui.label("DISCARD");
                        ui.separator();

                        ui.add(
                            Button::new(
                                RichText::new(format!("{}\n{}", discard, discard.color()))
                                    .color(egui::color::Color32::from_rgb(0, 0, 0))
                                    .size(25.),
                            )
                            .fill(match discard.color() {
                                Color::Red => egui::color::Color32::from_rgb(255, 0, 0),
                                Color::Green => egui::color::Color32::from_rgb(0, 255, 0),
                                Color::Yellow => egui::color::Color32::from_rgb(255, 255, 0),
                                Color::Blue => egui::color::Color32::from_rgb(0, 125, 255),
                                Color::None => egui::color::Color32::from_rgb(70, 70, 70),
                            }),
                        );
                    },
                );
            });
        });

        egui::Context::request_repaint(ctx);
    }
}

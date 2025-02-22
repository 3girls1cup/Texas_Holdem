use cosmwasm_std::Timestamp;
use secret_toolkit_permit::Permit;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::{Card, GameState};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StartGamePlayer {
    pub username: String,
    pub player_id: Uuid,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ExecuteMsg {
    StartGame {
        table_id: u32,
        hand_ref: u32,
        players: Vec<StartGamePlayer>,
        prev_hand_showdown_players: Vec<Uuid>, // player_ids of players who showed their cards in the last hand
    },
    CommunityCards {
        table_id: u32,
        game_state: GameState,
    },
    Showdown {
        table_id: u32,
        game_state: GameState,
        show_cards: Vec<String>, // player_ids of players whos cards are shown
    },
    Random {
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    WithPermit {
        permit: Permit,
        query: QueryWithPermit,
    },
    CommunityCards { table_id: u32, game_state: GameState, secret_key: u64 },
    Showdown { 
        table_id: u32, 
        flop_secret: Option<u64>,
        turn_secret: Option<u64>,
        river_secret: Option<u64>,
        players_secrets: Vec<u64>,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    PlayerPrivateData { table_id: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerDataResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub hand: Vec<Card>,
    pub hand_seed: u64,
    pub flop_secret: u64,
    pub turn_secret: u64,
    pub river_secret: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]  // Helps with JSON representation
pub enum ResponsePayload {
    StartGame(StartGameResponse),
    CommunityCards(CommunityCardsResponse),
    Showdown(ShowdownResponse),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StartGameResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub players: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommunityCardsResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub game_state: GameState,
    pub community_cards: Vec<Card>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShowdownResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub players_cards: Vec<(Uuid, Vec<Card>)>,
    pub community_cards: Option<Vec<Card>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ShowdownPlayer {
    pub username: String,
    pub hand: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LastHandLogResponse {
    pub showdown_players: Vec<ShowdownPlayer>, 
    pub community_cards: Vec<String>,
    pub flop_retrieved_at: Option<Timestamp>,
    pub turn_retrieved_at: Option<Timestamp>,
    pub river_retrieved_at: Option<Timestamp>,
    pub showdown_retrieved_at: Option<Timestamp>,
}

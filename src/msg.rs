use schemars::JsonSchema;
use secret_toolkit_permit::Permit;
use serde::{Deserialize, Serialize};

use crate::state::GameState;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    // pub gateway_address: Addr,
    // pub gateway_hash: String,
    // pub gateway_key: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StartGame {
        table_id: u32,
        hand_ref: u32,
        players: Vec<String>,// (userId, public_key)
        folded_win: bool,
    },
    CommunityCards {
        table_id: u32,
        game_state: GameState,
    },
    Showdown {
        table_id: u32,
        all_in_showdown: bool,
        show_cards: Vec<String>, // userId of players whos cards are shown
    },
    Random {
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryWithPermit {
    PlayerPrivateData { table_id: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PlayerDataResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub hand: Vec<u8>,
    pub hand_seed: u64,
    pub flop_secret: u64,
    pub turn_secret: u64,
    pub river_secret: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]  // Helps with JSON representation
pub enum ResponsePayload {
    StartGame(StartGameResponse),
    CommunityCards(CommunityCardsResponse),
    Showdown(ShowdownResponse),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StartGameResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub players: Vec<String>,
    pub folded_win: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CommunityCardsResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub game_state: GameState,
    pub community_cards: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ShowdownResponse {
    pub table_id: u32,
    pub hand_ref: u32,
    pub all_in_showdown: bool,
    pub players_cards: Vec<(String, Vec<u8>)>,
    pub community_cards: Option<Vec<u8>>,
}

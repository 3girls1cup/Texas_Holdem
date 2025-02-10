use std::collections::HashMap;

use schemars::JsonSchema;
use secret_toolkit_serialization::{Bincode2, Serde};
use serde::{Deserialize, Serialize};
use cosmwasm_std::Storage;
use cosmwasm_storage::{bucket, prefixed, Bucket, PrefixedStorage};

pub const PREFIX_TABLES: &[u8] = b"tables";

pub fn init_table_store(storage: &mut dyn Storage) -> Bucket<PokerTable> {
    bucket(storage, PREFIX_TABLES)
}

pub fn save_table(storage: &mut dyn Storage, table_id: u32, table: &PokerTable) {
    let mut table_store = init_table_store(storage);
    table_store.save(&table_id.to_be_bytes(), table).unwrap();
}

pub fn load_table(storage: &mut dyn Storage, table_id: u32) -> Option<PokerTable> {
    let table_store = init_table_store(storage);
    match table_store.may_load(&table_id.to_be_bytes()) {
        Ok(Some(table)) => Some(table),
        Ok(None) => None,
        Err(_) => None,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PlayerCards {
    pub hole_cards: Vec<u8>, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CommunityCards {
    pub flop: Vec<u8>, 
    pub turn: u8, 
    pub river: u8, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PokerTable {
    pub game_state: GameState,
    pub player_cards: HashMap<u8, PlayerCards>, 
    pub community_cards: CommunityCards, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameState {
    GameStart,
    Flop,
    Turn,
    River,
    EndGame,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Card(u8);

impl Card {
    pub fn new(suit: u8, rank: u8) -> Self {
        assert!(suit < 4, "Invalid suit");
        assert!(rank >= 1 && rank <= 13, "Invalid rank");
        Card((suit << 4) | rank)
    }

    pub fn suit(&self) -> u8 {
        self.0 >> 4
    }

    pub fn rank(&self) -> u8 {
        self.0 & 0b1111
    }

    pub fn to_bytes(&self) -> u8 {
        self.0
    }

    pub fn from_bytes(byte: u8) -> Self {
        Card(byte)
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    pub fn new() -> Self {
        let mut cards = Vec::new();
        for suit in 0..4 {
            for rank in 1..=13 {
                cards.push(Card::new(suit, rank));
            }
        }
        Deck { cards }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.cards.iter().map(|card| card.0).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let cards = bytes.iter().map(|&b| Card(b)).collect();
        Deck { cards }
    }
}


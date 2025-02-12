use schemars::JsonSchema;
use secret_toolkit_serialization::{Bincode2, Json};
use secret_toolkit_storage::{Item, Keymap, KeymapBuilder, WithoutIter};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, StdError, StdResult, Storage};


pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";

pub static CONFIG_KEY: Item<Config> = Item::new(b"config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub contract_address: Addr,
}

pub static TABLES_STORE: Keymap<u32, PokerTable, Json, WithoutIter> =
            KeymapBuilder::new(b"tables").without_iter().build();

pub fn save_table(storage: &mut dyn Storage, key: u32, item: &PokerTable) -> StdResult<()> {
    TABLES_STORE.insert(storage, &key, item).map_err(|err| {
        StdError::generic_err(format!("Failed to save table: {}", err))
    })
}

pub fn load_table(storage: &dyn Storage, key: u32) -> Option<PokerTable> {
    TABLES_STORE.get(storage, &key)
}

pub fn delete_table(storage: &mut dyn Storage, key: u32) -> StdResult<()> {
    TABLES_STORE.remove(storage, &key).map_err(|err| {
        StdError::generic_err(format!("Failed to delete table: {}", err))
    })
}

pub static PLAYER_SEED_STORE: Keymap<String, u64, Bincode2, WithoutIter> =
            KeymapBuilder::new(b"seeds").without_iter().build();

pub fn save_seed(storage: &mut dyn Storage, key: String, item: &u64) -> StdResult<()> {
    PLAYER_SEED_STORE.insert(storage, &key, item).map_err(|err| {
        StdError::generic_err(format!("Failed to save seed: {}", err))
    })
}

pub fn load_seed(storage: &dyn Storage, key: String) -> Option<u64> {
    PLAYER_SEED_STORE.get(storage, &key)
}

#[cfg(test)]
mod tests {
use cosmwasm_std::{testing::MockStorage, StdError};

use super::*;



    #[test]
    fn test_keymap() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let key = 1u32;
        let item = PokerTable {
            game_state: GameState::PreFlop,
            player_cards: vec![(
                "SDER".to_string(), 
                PlayerCards { hole_cards: vec![] }
            )],
            community_cards: CommunityCards {
                flop: vec![],
                turn: 0,
                river: 0,
            },
        };
        TABLES_STORE.insert(&mut storage, &key, &item).map_err(|err| {
            StdError::generic_err(format!("Failed to save table: {}", err))
        })

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
    pub player_cards: Vec<(String, PlayerCards)>,  // player's public address as a key
    pub community_cards: CommunityCards, 
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameState {
    PreFlop,
    Flop,
    Turn,
    River,
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


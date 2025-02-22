use schemars::JsonSchema;
use secret_toolkit_serialization::{Bincode2, Json};
use secret_toolkit_storage::{Item, Keymap, KeymapBuilder, WithoutIter};
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Addr, StdError, StdResult, Storage};

use crate::contract::encrypt_cards;


pub const PREFIX_REVOKED_PERMITS: &str = "revoked_permits";

pub static COUNTER_KEY: Item<u128> = Item::new(b"counter");

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

#[cfg(test)]
mod tests {
use cosmwasm_std::{testing::MockStorage, StdError};

use super::*;
    #[test]
    fn test_keymap() -> StdResult<()> {
        let mut storage = MockStorage::new();
        let key = 1u32;
        let community_cards = CommunityCards {
            flop: vec![],
            turn: 0,
            river: 0,
        };

        let item = PokerTable {
            game_state: GameState::PreFlop,
            hand_ref: 1,
            players: vec![Player {
                public_key: "public_key".to_string(),
                encrypted_hand: PlayerCards { hole_cards: vec![] },
                decrypted_hand: PlayerCards { hole_cards: vec![] },
                hand_seed: 0,
                flop_seed_share: 0,
                turn_seed_share: 0,
                river_seed_share: 0,
            }],
            community_cards: CommunityCardsWrapper {
                encrypted: community_cards.clone(),
                decrypted: community_cards.clone(),
                flop_secret: 0,
                turn_secret: 0,
                river_secret: 0,
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

impl CommunityCards {
    pub fn encrypt(&self, flop_secret: u64, turn_secret: u64, river_secret: u64) -> CommunityCards {
        CommunityCards {
            flop: encrypt_cards(flop_secret, self.flop.clone()),
            turn: encrypt_cards(turn_secret, self.turn),
            river: encrypt_cards(river_secret, self.river),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct PokerTable {
    pub game_state: GameState,
    pub hand_ref: u32,
    pub players: Vec<Player>,
    pub community_cards: CommunityCardsWrapper,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CommunityCardsWrapper {
    pub encrypted: CommunityCards,
    pub decrypted: CommunityCards,
    pub flop_secret: u64,
    pub turn_secret: u64,
    pub river_secret: u64,
}

impl CommunityCardsWrapper {
    pub fn new(community_cards: CommunityCards, flop_secret: u64, turn_secret: u64, river_secret: u64) -> Self {
        CommunityCardsWrapper {
            encrypted: community_cards.encrypt(flop_secret, turn_secret, river_secret),
            decrypted: community_cards,
            flop_secret,
            turn_secret,
            river_secret,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Player {
    pub public_key: String,
    pub encrypted_hand: PlayerCards,
    pub decrypted_hand: PlayerCards,
    pub hand_seed: u64,
    pub flop_seed_share: u64,
    pub turn_seed_share: u64,
    pub river_seed_share: u64,
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

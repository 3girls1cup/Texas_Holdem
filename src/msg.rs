use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::GameState;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    StartGame {
        table_id: u32,
        players: Vec<(u8, String, u64)>,// seat_index -> (public_key, random_seed)
    },
    CommunityCards {
        table_id: u32,
        game_state: GameState,
    },
}

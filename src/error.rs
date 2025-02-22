use core::error;

use cosmwasm_std::StdError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::GameState;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    // let thiserror implement From<StdError> for you
    Std(#[from] StdError),

    #[error("Unauthorized")]
    // issued when message sender != owner
    Unauthorized {},

    #[error("Game state error in method {method} for table {table_id}: needed {needed:?}, but got {actual:?}")]
    // issued when game state is invalid
    GameStateError {
        method: String,
        table_id: u32,
        needed: Option<GameState>,
        actual: GameState,
    },

    #[error("Player {player} not found in table {table_id}")]
    // issued when player is not found
    PlayerNotFound { table_id: u32, player: String },

    #[error("Table {table_id} not found")]
    // issued when table is not found
    TableNotFound { table_id: u32 },

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

#[derive(Error, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryError {

    #[error("Player {player} not found in table {table_id}")]
    // issued when player is not found
    PlayerNotFound { table_id: u32, player: String },

    #[error("Table {table_id} not found")]
    // issued when table is not found
    TableNotFound { table_id: u32 },

    #[error("Invalid game state: {game_state:?}")]
    // issued when game state is invalid
    InvalidGameState { game_state: GameState },
    
    #[error("Invalid viewing secret {key}")]
    // issued when viewing key is invalid
    InvalidViewingKey { key: u64 },
}

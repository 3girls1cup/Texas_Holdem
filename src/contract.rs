use std::collections::HashSet;

use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use secret_toolkit_crypto::hkdf_sha_512;
use secret_toolkit_permit::{validate, Permit};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{ContractError, QueryError};
use crate::msg::{
    CommunityCardsResponse, ExecuteMsg, InstantiateMsg, LastHandLogResponse, QueryMsg, QueryWithPermit, ResponsePayload, ShowdownPlayer, ShowdownResponse, StartGamePlayer, StartGameResponse
};
use crate::state::{
    load_table, save_table, Card, CommunityCards, Config, Deck, Flop, GameState,
    Player, PokerTable, River, Turn, CONFIG_KEY, COUNTER_KEY, PREFIX_REVOKED_PERMITS,
};

// Constants
const MIN_PLAYERS: usize = 2;
const MAX_PLAYERS: usize = 9;
const COMMUNITY_CARD_PHASES: usize = 3;
const SECRET_LENGTH: usize = 64;
const RANDOM_SEED_SIZE: usize = 16;
const RESPONSE_KEY: &str = "response";

// Helper modules
mod helpers {
    use super::*;

    pub fn generate_random_number(env: &Env, counter: &mut u128) -> StdResult<u64> {
        let secret = hkdf_sha_512(
            &Some(vec![0u8; SECRET_LENGTH]),
            &env.block.random.as_ref().unwrap(),
            &counter.to_le_bytes(),
            SECRET_LENGTH,
        )?;

        *counter += 1;
        Ok(u64::from_le_bytes(secret[..8].try_into().unwrap()))
    }

    pub fn additive_secret_sharing(
        env: &Env,
        players: usize,
        secret: u64,
        counter: &mut u128,
    ) -> StdResult<Vec<u64>> {
        let mut shares = Vec::with_capacity(players);
        let mut sum: u64 = 0;

        for _ in 0..(players - 1) {
            let share = generate_random_number(env, counter)?;
            shares.push(share);
            sum = sum.wrapping_add(share);
        }

        shares.push(secret.wrapping_sub(sum));
        Ok(shares)
    }

    pub fn shuffle_deck(deck: &mut Deck, seed: u64) {
        let mut rng = Sha256::new();
        let mut deck_len = deck.cards.len();

        while deck_len > 1 {
            deck_len -= 1;
            rng.update(&seed.to_le_bytes());
            rng.update(&(deck_len as u64).to_le_bytes());

            let hash = rng.finalize_reset();
            let random_value = u64::from_le_bytes(hash[..8].try_into().unwrap());
            let random_index = (random_value as usize) % (deck_len + 1);

            deck.cards.swap(deck_len, random_index);
        }
    }
}

// State management
mod state_utils {
    use super::*;

    pub fn load_table_or_error(
        storage: &dyn cosmwasm_std::Storage,
        table_id: u32,
    ) -> Result<PokerTable, ContractError> {
        load_table(storage, table_id).ok_or(ContractError::TableNotFound { table_id })
    }
}

// Query handlers
mod query_handlers {
    use super::*;

    pub fn handle_permit_query(
        deps: Deps,
        permit: Permit,
        query: QueryWithPermit,
    ) -> StdResult<Binary> {
        let config = CONFIG_KEY.load(deps.storage)?;
        let viewer = validate(
            deps,
            PREFIX_REVOKED_PERMITS,
            &permit,
            config.contract_address.to_string(),
            None,
        )?;

        match query {
            QueryWithPermit::PlayerPrivateData { table_id } => {
                to_binary(&query_player_private_data(deps, table_id, viewer))
            }
        }
    }

    pub fn query_player_private_data(
        deps: Deps,
        table_id: u32,
        pub_key: String,
    ) -> Result<Player, QueryError> {
        let table =
            load_table(deps.storage, table_id).ok_or(QueryError::TableNotFound { table_id })?;

        table
            .players
            .iter()
            .find(|p| p.public_key == pub_key)
            .cloned()
            .ok_or(QueryError::PlayerNotFound {
                table_id,
                player: pub_key,
            })
    }

    pub fn query_community_cards(
        deps: Deps,
        table_id: u32,
        game_state: GameState,
        secret_key: u64,
    ) -> Result<CommunityCardsResponse, QueryError> {
        let table =
            load_table(deps.storage, table_id).ok_or(QueryError::TableNotFound { table_id })?;

        let (stored_key, cards) = match game_state {
            GameState::Flop => (
                table.community_cards.flop.secret,
                table.community_cards.flop.cards,
            ),
            GameState::Turn => (
                table.community_cards.turn.secret,
                vec![table.community_cards.turn.card],
            ),
            GameState::River => (
                table.community_cards.river.secret,
                vec![table.community_cards.river.card],
            ),
            _ => return Err(QueryError::InvalidGameState { game_state }),
        };

        if stored_key != secret_key {
            return Err(QueryError::InvalidViewingKey { key: secret_key });
        }

        Ok(CommunityCardsResponse {
            table_id,
            hand_ref: table.hand_ref,
            game_state,
            community_cards: cards,
        })
    }

    pub fn query_showdown(
        deps: Deps,
        table_id: u32,
        flop_secret: Option<u64>,
        turn_secret: Option<u64>,
        river_secret: Option<u64>,
        players_secrets: Vec<u64>,
    ) -> Result<ShowdownResponse, QueryError> {
        let table =
            load_table(deps.storage, table_id).ok_or(QueryError::TableNotFound { table_id })?;

        let mut community_cards = Vec::new();

        if let Some(secret) = flop_secret {
            if table.community_cards.flop.secret != secret {
                return Err(QueryError::InvalidViewingKey { key: secret });
            }
            community_cards.extend(table.community_cards.flop.cards.clone());
        }

        if let Some(secret) = turn_secret {
            if table.community_cards.turn.secret != secret {
                return Err(QueryError::InvalidViewingKey { key: secret });
            }
            community_cards.push(table.community_cards.turn.card);
        }

        if let Some(secret) = river_secret {
            if table.community_cards.river.secret != secret {
                return Err(QueryError::InvalidViewingKey { key: secret });
            }
            community_cards.push(table.community_cards.river.card);
        }

        let players_cards = players_secrets
            .iter()
            .map(|secret| {
                table
                    .players
                    .iter()
                    .find(|player| &player.hand_secret == secret)
                    .map(|player| (player.player_id.clone(), player.hand.clone()))
                    .ok_or_else(|| QueryError::SecretNotFound {
                        val: secret.to_string(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ShowdownResponse {
            table_id,
            hand_ref: table.hand_ref,
            players_cards,
            community_cards: Some(community_cards),
        })
    }
}

// Execute handlers
mod execute_handlers {
    use super::{state_utils::load_table_or_error, *};

    pub fn handle_start_game(
        deps: DepsMut,
        env: Env,
        table_id: u32,
        hand_ref: u32,
        players_info: Vec<StartGamePlayer>,
        prev_hand_showdown_players: Vec<Uuid>,
    ) -> Result<Response, ContractError> {
        validate_players(&players_info)?;
        let previous_hand_log = create_previous_hand_log(deps.as_ref(), table_id, prev_hand_showdown_players)?;
        let mut counter = COUNTER_KEY.load(deps.storage)?;
        let mut deck = initialize_deck(&env, &mut counter)?;
        let player_cards = distribute_player_cards(&mut deck, &players_info);
        let mut secrets = Vec::with_capacity(COMMUNITY_CARD_PHASES);
        let community_cards =
            generate_community_cards(&env, &mut counter, &mut secrets, &mut deck, players_info.len())?;
        let players = create_players(
            players_info,
            player_cards,
            &secrets,
            &env,
            &mut counter,
        )?;

        let table = PokerTable {
            hand_ref,
            players,
            community_cards,
            showdown_retrieved_at: None,
        };

        save_table(deps.storage, table_id, &table)?;
        COUNTER_KEY.save(deps.storage, &counter)?;

        create_start_game_response(
            table_id,
            hand_ref,
            &table.players,
            previous_hand_log,
        )
    }

    fn validate_players(players_info: &[StartGamePlayer]) -> Result<(), ContractError> {
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players_info.len()) {
            return Err(ContractError::InvalidPlayerCount {
                count: players_info.len(),
            });
        }

        let unique_keys: HashSet<_> = players_info.iter().map(|p| &p.public_key).collect();

        if unique_keys.len() != players_info.len() {
            return Err(ContractError::DuplicatePublicKeys {});
        }

        Ok(())
    }

    fn initialize_deck(env: &Env, counter: &mut u128) -> Result<Deck, ContractError> {
        let mut deck = Deck::new();
        let seed = helpers::generate_random_number(env, counter)?;
        helpers::shuffle_deck(&mut deck, seed);
        Ok(deck)
    }

    fn distribute_player_cards(
        deck: &mut Deck,
        players: &[StartGamePlayer],
    ) -> Vec<(String, Vec<Card>)> {
        let mut deck_iter = deck.cards.iter();
        players
            .iter()
            .map(|player| {
                (
                    player.public_key.clone(),
                    vec![
                        deck_iter.next().unwrap().clone(),
                        deck_iter.next().unwrap().clone(),
                    ],
                )
            })
            .collect()
    }

    fn generate_community_cards(
        env: &Env,
        counter: &mut u128,
        secrets: &mut Vec<(u64, Vec<u64>)>,
        deck: &mut Deck,
        player_count: usize,
    ) -> Result<CommunityCards, ContractError> {
        let mut deck_iter = deck.cards.iter();

        for _ in 0..COMMUNITY_CARD_PHASES {
            let secret = helpers::generate_random_number(env, counter)?;
            let shares = helpers::additive_secret_sharing(env, player_count, secret, counter)?;
            secrets.push((secret, shares));
        }

        Ok(CommunityCards {
            flop: Flop {
                cards: collect_cards(&mut deck_iter, 3),
                secret: secrets[0].0,
                retrieved_at: None,
            },
            turn: Turn {
                card: deck_iter.next().unwrap().clone(),
                secret: secrets[1].0,
                retrieved_at: None,
            },
            river: River {
                card: deck_iter.next().unwrap().clone(),
                secret: secrets[2].0,
                retrieved_at: None,
            },
        })
    }

    fn collect_cards(iter: &mut std::slice::Iter<Card>, count: usize) -> Vec<Card> {
        (0..count).filter_map(|_| iter.next().cloned()).collect()
    }

    fn create_players(
        players_info: Vec<StartGamePlayer>,
        player_cards: Vec<(String, Vec<Card>)>,
        secrets: &[(u64, Vec<u64>)],
        env: &Env,
        counter: &mut u128,
    ) -> Result<Vec<Player>, ContractError> {

        players_info
            .into_iter()
            .zip(player_cards)
            .enumerate()
            .map(|(i, (info, (_, cards)))| {
                Ok(Player {
                    username: info.username,
                    player_id: info.player_id,
                    public_key: info.public_key,
                    hand: cards,
                    hand_secret: helpers::generate_random_number(env, counter)?,
                    flop_secret_share: secrets[0].1[i],
                    turn_secret_share: secrets[1].1[i],
                    river_secret_share: secrets[2].1[i],
                })
            })
            .collect()
    }

    fn create_start_game_response(
        table_id: u32,
        hand_ref: u32,
        players: &[Player],
        previous_hand_log: Option<LastHandLogResponse>,
    ) -> Result<Response, ContractError> {
        let response = ResponsePayload::StartGame(StartGameResponse {
            table_id,
            hand_ref,
            players: players.iter().map(|p| p.username.clone()).collect(),
        });
        let mut res = create_plaintext_response(RESPONSE_KEY.to_string(), response)?;

        if let Some(previous_hand_log) = previous_hand_log {
            res = res.add_attribute_plaintext("previous_hand_log", serialize_response(ResponsePayload::LastHand(Some(previous_hand_log)))?);
        }
        Ok(res)
    }

    fn create_previous_hand_log(deps: Deps, table_id: u32, showdown_player_ids: Vec<Uuid>) -> Result<Option<LastHandLogResponse>, ContractError> {
        let table = load_table(deps.storage, table_id);
        let previous_hand_log = if table.is_some() {
            let table = table.unwrap();
            
            Some(LastHandLogResponse {
                showdown_players: showdown_player_ids.iter().map(|player_id| {
                    let player = table.players.iter().find(|player| &player.player_id == player_id).unwrap();
                    ShowdownPlayer {
                        username: player.username.clone(),
                        hand: player.hand.iter().map(|card| card.to_string()).collect(),
                    }
                }).collect(),
                community_cards: [table.community_cards.flop.cards.iter().map(|card| card.to_string()).collect(), vec![table.community_cards.turn.card.to_string()], vec![table.community_cards.river.card.to_string()]].concat(),
                flop_retrieved_at: table.community_cards.flop.retrieved_at,
                turn_retrieved_at: table.community_cards.turn.retrieved_at,
                river_retrieved_at: table.community_cards.river.retrieved_at,
                showdown_retrieved_at: table.showdown_retrieved_at,
            })
        } else {
            None
        };

        Ok(previous_hand_log)
    }

    fn create_plaintext_response(
        key: String,
        response: ResponsePayload
    ) -> Result<Response, ContractError> {
        Ok(Response::new().add_attribute_plaintext(key, serialize_response(response)?))
    }

    fn serialize_response(response: ResponsePayload) -> Result<String, ContractError> {
        match serde_json_wasm::to_string(&response) {
            Ok(json) => Ok(json),
            Err(e) => Err(ContractError::SerializationFailed {
                error: e.to_string(),
            }),
        }
    }


    pub fn handle_community_cards(
        deps: DepsMut,
        env: Env,
        table_id: u32,
        game_state: GameState,
    ) -> Result<Response, ContractError> {
        let mut table = load_table_or_error(deps.storage, table_id)?;
        
        let cards = match game_state {
            GameState::Flop => {
                table.community_cards.flop.retrieved_at = Some(env.block.time);
                Some(table.community_cards.flop.cards.clone())
            }
            GameState::Turn => {
                table.community_cards.turn.retrieved_at = Some(env.block.time);
                Some(vec![table.community_cards.turn.card.clone()])
            }
            GameState::River => {
                table.community_cards.river.retrieved_at = Some(env.block.time);
                Some(vec![table.community_cards.river.card.clone()])
            }
            _ => {
                return Err(ContractError::GameStateError {
                    method: "distribute_community_cards".to_string(),
                    table_id,
                    game_state: Some(game_state),
                })
            }
        };

        // Log the retrieved_at time
        save_table(deps.storage, table_id, &table)?;

        let response = ResponsePayload::CommunityCards(CommunityCardsResponse {
            table_id,
            hand_ref: table.hand_ref,
            game_state,
            community_cards: cards.unwrap(),
        });

        create_plaintext_response(RESPONSE_KEY.to_string(), response)
    }

    pub fn handle_showdown(
        deps: DepsMut,
        env: Env,
        table_id: u32,
        game_state: GameState,
        showdown_player_ids: Vec<Uuid>,
    ) -> Result<Response, ContractError> {
        let mut table = load_table(deps.storage, table_id)
            .ok_or_else(|| ContractError::TableNotFound { table_id })?;

        let mut player_hands: Vec<(Uuid, Vec<Card>)> = Vec::new();

        for player_id in showdown_player_ids.iter() {
            let players = table
                .players
                .iter()
                .find(|player| &player.player_id == player_id);

            if let Some(player) = players {
                player_hands.push((player.player_id.clone(), player.hand.clone()));
            } else {
                return Err(ContractError::PlayerNotFound {
                    table_id,
                    player: player_id.to_string(),
                });
            }
        }

        let response = ResponsePayload::Showdown(ShowdownResponse {
            table_id,
            hand_ref: table.hand_ref,
            players_cards: player_hands,
            community_cards: handle_all_in_showdown(&table.community_cards, game_state),
        });

        // Log the showdown retrieval time
        table.showdown_retrieved_at = Some(env.block.time);
        save_table(deps.storage, table_id, &table)?;

        create_plaintext_response(RESPONSE_KEY.to_string(), response)
    }

    fn handle_all_in_showdown(
        community_cards: &CommunityCards,
        game_state: GameState,
    ) -> Option<Vec<Card>> {
        match game_state {
            GameState::PreFlop => {
                let mut cards = community_cards.flop.cards.clone();
                cards.push(community_cards.turn.card.clone());
                cards.push(community_cards.river.card.clone());
                Some(cards)
            }
            GameState::Flop => Some(vec![
                community_cards.turn.card.clone(),
                community_cards.river.card.clone(),
            ]),
            GameState::Turn => Some(vec![community_cards.river.card.clone()]),
            _ => return None,
        }
    }
}

// Contract entry points
#[entry_point]
pub fn instantiate(deps: DepsMut, env: Env, info: MessageInfo, _msg: InstantiateMsg,) -> Result<Response, StdError> {
    let config = Config {
        owner: info.sender,
        contract_address: env.contract.address.clone(),
    };

    let counter = init_counter(&env)?;

    CONFIG_KEY.save(deps.storage, &config)?;
    COUNTER_KEY.save(deps.storage, &counter)?;

    Ok(Response::default())
}

fn init_counter(env: &Env) -> StdResult<u128> {
    let seed = env
        .block
        .random
        .as_ref()
        .ok_or(StdError::generic_err("No random seed available"))?;
    let seed_bytes: [u8; RANDOM_SEED_SIZE] = seed[..RANDOM_SEED_SIZE]
        .try_into()
        .map_err(|_| StdError::generic_err("Failed to convert seed to array"))?;
    Ok(u128::from_le_bytes(seed_bytes) % 1000)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG_KEY.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::StartGame {
            table_id,
            hand_ref,
            players,
            prev_hand_showdown_players,
        } => execute_handlers::handle_start_game(
            deps,
            env,
            table_id,
            hand_ref,
            players,
            prev_hand_showdown_players,
        ),
        ExecuteMsg::CommunityCards {
            table_id,
            game_state,
        } => execute_handlers::handle_community_cards(deps, env, table_id, game_state),
        ExecuteMsg::Showdown {
            table_id,
            game_state,
            showdown_player_ids,
        } => execute_handlers::handle_showdown(deps, env, table_id, game_state, showdown_player_ids),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::WithPermit { permit, query } => {
            query_handlers::handle_permit_query(deps, permit, query)
        }
        QueryMsg::CommunityCards {
            table_id,
            game_state,
            secret_key,
        } => to_binary(&query_handlers::query_community_cards(
            deps, table_id, game_state, secret_key,
        )),
        QueryMsg::Showdown {
            table_id,
            flop_secret,
            turn_secret,
            river_secret,
            players_secrets,
        } => to_binary(&query_handlers::query_showdown(
            deps,
            table_id,
            flop_secret,
            turn_secret,
            river_secret,
            players_secrets,
        )),
    }
}

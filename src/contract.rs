use std::collections::HashSet;
use secret_toolkit_permit::{validate, Permit};
use sha2::{Digest, Sha256};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};

use crate::error::ContractError;
use crate::msg::{CommunityCardsResponse, ExecuteMsg, HandResponse, InstantiateMsg, QueryMsg, QueryWithPermit, ResponsePayload, ShowdownResponse, StartGameResponse};
use crate::state::{ delete_table, load_table, save_table, CommunityCards, Config, Deck, GameState, PlayerCards, PokerTable, CONFIG_KEY, PREFIX_REVOKED_PERMITS};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let config = Config {
        owner: info.sender,
        contract_address: env.contract.address,
    };

    CONFIG_KEY.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {

    let owner_adress = CONFIG_KEY.load(deps.storage)?.owner;

    if info.sender != owner_adress {
        return Err(ContractError::Unauthorized {});
    }

    match msg {
        ExecuteMsg::StartGame {table_id,players } => start_game(deps, env, table_id, players),
        ExecuteMsg::CommunityCards {table_id, game_state} => distribute_community_cards(deps, table_id, game_state),
        ExecuteMsg::Showdown {table_id, all_in_showdown, show_cards} => showdown(deps, env, table_id,all_in_showdown, show_cards),
    }
}



fn showdown(    
    deps: DepsMut,
    _env: Env,
    table_id: u32,
    all_in_showdown: bool,
    show_cards: Vec<String>,
) -> Result<Response, ContractError> {
    let table = load_table(deps.storage, table_id)
        .ok_or_else(|| ContractError::TableNotFound { table_id })?;

    if !all_in_showdown && table.game_state != GameState::River {
        return Err(ContractError::GameStateError { method: "showdown".to_string(), table_id, needed: Some(GameState::River), actual: table.game_state });
    }

    let mut player_hands: Vec<(String, Vec<u8>)> = Vec::new();

    for pub_key in show_cards.iter() {
        let player_cards = table.player_cards.iter().find(|(addr, _)| addr == pub_key);
    
        if let Some((addr, cards)) = player_cards {
            player_hands.push((addr.clone(), cards.hole_cards.clone()));
        } else {
            return Err(ContractError::PlayerNotFound { table_id, player: pub_key.clone() });
        }
    }

    delete_table(deps.storage, table_id)?;

    let response = ResponsePayload::Showdown(ShowdownResponse {
        table_id,
        all_in_showdown,
        players_cards: player_hands,
        community_cards: handle_all_in_showdown(&table, all_in_showdown), // Fixed borrowing
        error: None,
    });

    let json_response = serde_json_wasm::to_string(&response)
        .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    Ok(Response::new().add_attribute_plaintext("response", json_response))
}

fn handle_all_in_showdown(table: &PokerTable, all_in_showdown: bool) -> Option<Vec<u8>> {
    if all_in_showdown {
        match table.game_state {
            GameState::PreFlop => {
                let mut cards = table.community_cards.flop.clone();
                cards.push(table.community_cards.turn);
                cards.push(table.community_cards.river);
                Some(cards)
            }
            GameState::Flop => Some(vec![table.community_cards.turn, table.community_cards.river]),
            GameState::Turn => Some(vec![table.community_cards.river]),
            _ => None,
        }
    } else {
        None
    }
}

fn distribute_community_cards(
    deps: DepsMut,
    table_id: u32,
    game_state: GameState,
) -> Result<Response, ContractError> {

    let table = load_table(deps.storage, table_id)
        .ok_or_else(|| ContractError::TableNotFound { table_id })?;

    let cards = match (table.game_state.clone(), &game_state) {
        (GameState::PreFlop, GameState::Flop) => Some(table.community_cards.flop.clone()), 
        (GameState::Flop, GameState::Turn) => Some(vec![table.community_cards.turn]), 
        (GameState::Turn, GameState::River) => Some(vec![table.community_cards.river]), 
        _ => return Err(ContractError::GameStateError { method: "distribute_community_cards".to_string(), table_id, needed: get_needed_game_state(game_state), actual: table.game_state.clone() }),
    };

    
    let mut updated_table = table.clone();
    let game_state_clone = game_state.clone();
    updated_table.game_state = game_state;

    save_table(deps.storage, table_id, &updated_table)?;
    
    let response = ResponsePayload::CommunityCards(CommunityCardsResponse {
        table_id,
        game_state: game_state_clone,
        community_cards: cards.unwrap(),
        error: None,
    });

    let json_response = serde_json_wasm::to_string(&response)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    Ok(Response::new()
        .add_attribute_plaintext("response", json_response))
}


fn get_needed_game_state(requested_game_state: GameState) -> Option<GameState> {
    match requested_game_state {
        GameState::Flop => Some(GameState::PreFlop),
        GameState::Turn => Some(GameState::Flop),
        GameState::River => Some(GameState::Turn),
        _ => None,
    }
}

fn start_game(
    deps: DepsMut,
    env: Env,
    table_id: u32,
    players: Vec<String>,
) -> Result<Response, ContractError> {

    let table = load_table(deps.storage, table_id);

    // If the table already exists, it means it hasn't ended yet
    if !table.is_none() {
        let table = table.unwrap();
        return Err(ContractError::GameStateError { method: "start_game".to_string(), table_id, needed: None, actual: table.game_state });
    }

    if players.len() < 2 || players.len() > 9 {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Number of players must be between 2 and 9")));
    }

    let unique_players: HashSet<String> = players.iter().map(|addr| addr.clone()).collect();
    if unique_players.len() != players.len() {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Duplicated public keys")));
    }

    let random_number = generate_random_number(&env)?;

    let mut deck = Deck::new();

    shuffle_deck(&mut deck, random_number);

    
    let mut player_cards: Vec<(String, PlayerCards)> = Vec::new();
    let mut deck_iter = deck.cards.iter();
    
    for pub_key in players.clone() {
        let hole_cards = vec![deck_iter.next().unwrap(), deck_iter.next().unwrap()];
        player_cards.push((pub_key, PlayerCards { hole_cards: hole_cards.iter().map(|card| card.to_bytes()).collect() }));
    }
    
    let flop = vec![deck_iter.next().unwrap(), deck_iter.next().unwrap(), deck_iter.next().unwrap()];
    let turn = deck_iter.next().unwrap();
    let river = deck_iter.next().unwrap();


    let community_cards = CommunityCards {
        flop: flop.iter().map(|card| card.to_bytes()).collect(),
        turn: turn.to_bytes(),
        river: river.to_bytes(),
    };

    let table = PokerTable {
        game_state: GameState::PreFlop,
        player_cards: player_cards.iter().map(|(s, p)| (s.clone(), p.clone())).collect(),
        community_cards,
    };

    save_table(deps.storage, table_id, &table)?;

    let response = ResponsePayload::StartGame(StartGameResponse {
        table_id,
        players,
        error: None,
    });

    let json_response = serde_json_wasm::to_string(&response)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

Ok(Response::new().add_attribute_plaintext("response", json_response))
}

fn generate_random_number(
    env: &Env,
) -> StdResult<u64> {
    let seed = env.block.random.as_ref().unwrap();
    let mut hasher = Sha256::new();

    hasher.update(seed.as_slice());
    
    let final_hash = hasher.finalize();
    let final_seed = u64::from_le_bytes(final_hash[..8].try_into().unwrap());

    Ok(final_seed)
}


fn shuffle_deck(deck: &mut Deck, final_seed: u64) {
    let mut deck_len = deck.cards.len();

    while deck_len > 1 {
        deck_len -= 1;
     
        let random_index = generate_derived_random_index(final_seed, deck_len as u64, deck_len);
    
        deck.cards.swap(deck_len, random_index);
    }
}

fn generate_derived_random_index(seed: u64, round: u64, max: usize) -> usize {
    let mut hasher = Sha256::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(&round.to_le_bytes());
    
    let hash = hasher.finalize();
    
    let random_value = u64::from_le_bytes(hash[..8].try_into().unwrap());
    
    (random_value as usize) % (max + 1)
}

#[entry_point]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::WithPermit { permit, query } => permit_queries(deps, permit, query),
    }
}

fn permit_queries(
    deps: Deps,
    permit: Permit,
    query: QueryWithPermit,
) -> StdResult<Binary> {
    // Validate permit content
    let config = CONFIG_KEY.load(deps.storage)?;

    let viewer = validate(
        deps,
        PREFIX_REVOKED_PERMITS,
        &permit,
        config.contract_address.to_string(),
        None,
    )?;

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::GetPlayerCards {table_id} => {
            to_binary(&get_hand(deps, table_id, viewer))
        }
    }
}

fn get_hand(deps: Deps, table_id: u32, pub_key: String) -> Option<HandResponse> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Some(HandResponse {
            cards: vec![],
            error: Some(format!("Table {} not found", table_id)),
        });
    }

    let table = table.unwrap();
    let player_cards = table.player_cards.iter().find(|(addr, _)| addr == &pub_key);

    if player_cards.is_none() {
        return Some(HandResponse {
            cards: vec![],
            error: Some(format!("Player with address {} not found for table {}", pub_key, table_id)),
        });
    }

    let player_cards = player_cards.unwrap().1.clone();

    Some(HandResponse {
        cards: player_cards.hole_cards,
        error: None,
    })
}
use std::collections::{HashMap, HashSet};
use chacha20_poly1305::{ChaCha20Poly1305, Key};
use sha2::{Digest, Sha256};
use cosmwasm_std::{
    entry_point, Addr, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{config, CommunityCards, Deck, GameState, PlayerCards, State};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    deps.api
        .debug(&format!("Contract was initialized by {}", info.sender));

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StartGame {table_id, players} => start_game(deps, env, table_id, players),
    }
}


pub fn start_game(
    deps: DepsMut,
    env: Env,
    table_id: u32,
    players: HashMap<u8, (Addr, u64)>,
) -> Result<Response, ContractError> {
    
    if players.len() < 2 || players.len() > 9 {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Le nombre de joueurs doit être entre 2 et 9")));
    }
    for &seat in players.keys() {
        if seat > 8 {
            return Err(ContractError::from(cosmwasm_std::StdError::generic_err(format!("Seat index {} invalide", seat))));
        }
    }
    let unique_players: HashSet<&Addr> = players.values().map(|(addr, _)| addr).collect();
    if unique_players.len() != players.len() {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Clés publiques dupliquées détectées")));
    }
    for (_, (_, seed)) in &players {
        if *seed == 0 {
            return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Un joueur a envoyé un seed invalide (0)")));
        }
    }

    
    let final_seed = generate_final_seed(&env, &players)?;

    
    let mut deck = Deck::new();
    shuffle_deck(&mut deck, final_seed);

    
    let mut player_cards: HashMap<u8, PlayerCards> = HashMap::new();
    let mut deck_iter = deck.cards.iter();
    
    for &seat in players.keys() {
        let hole_cards = vec![deck_iter.next().unwrap(), deck_iter.next().unwrap()];
        player_cards.insert(seat, PlayerCards { hole_cards: hole_cards.iter().map(|card| card.to_bytes()).collect() });
    }

    
    let flop = vec![deck_iter.next().unwrap(), deck_iter.next().unwrap(), deck_iter.next().unwrap()];
    let turn = deck_iter.next().unwrap();
    let river = deck_iter.next().unwrap();


    let community_cards = CommunityCards {
        flop: flop.iter().map(|card| card.to_bytes()).collect(),
        turn: turn.to_bytes(),
        river: river.to_bytes(),
    };

    
    let encrypted_cards = encrypt_player_cards(&players, &player_cards)?;

    
    let state = State {
        game_state: GameState::GameStart,
        owner: env.contract.address.clone(),
        player_cards,
        community_cards,
    };
    config(deps.storage).save(&state)?;

    deps.api.debug(&format!("🃏 Deck mélangé avec le seed final : {}", final_seed));

    Ok(Response::new()
        .add_attribute("action", "game_started")
        .add_attribute("table_id", table_id.to_string())
        .add_attribute("final_seed", final_seed.to_string())
        .add_attribute("encrypted_cards", format!("{:?}", encrypted_cards))) 
}


pub fn encrypt_player_cards(
    players: &HashMap<u8, (Addr, u64)>,
    player_cards: &HashMap<u8, PlayerCards>,
) -> StdResult<HashMap<u8, Vec<u8>>> {
    let mut encrypted_cards = HashMap::new();

    for (seat, _) in players {
        let cards = &player_cards[seat].hole_cards;
        encrypted_cards.insert(*seat, cards.clone());
    }

    Ok(encrypted_cards)
}


fn derive_encryption_key(addr: &Addr) -> [u8; 32] {
    let hash = Sha256::digest(addr.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash[..32]);
    key
}



pub fn generate_final_seed(
    env: &Env,
    players: &HashMap<u8, (Addr, u64)>,
) -> StdResult<u64> {
    
    let mut hasher = Sha256::new();

    
    for (seat, (_, seed)) in players {
        hasher.update(&seat.to_le_bytes()); 
        hasher.update(&seed.to_le_bytes()); 
    }

    
    if let Some(random_bytes) = &env.block.random {
        hasher.update(random_bytes.as_slice());
    } else {
        return Err(cosmwasm_std::StdError::generic_err("Random value from block is missing").into());
    }

    
    let final_hash = hasher.finalize();
    let final_seed = u64::from_le_bytes(final_hash[..8].try_into().unwrap());

    Ok(final_seed)
}


pub fn shuffle_deck(deck: &mut Deck, final_seed: u64) {
    let mut deck_len = deck.cards.len();

    while deck_len > 1 {
        deck_len -= 1;

        
        let random_index = generate_pseudo_random_index(final_seed, deck_len as u64, deck_len);

        
        deck.cards.swap(deck_len, random_index);
    }
}


fn generate_pseudo_random_index(seed: u64, round: u64, max: usize) -> usize {
    let mut hasher = Sha256::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(&round.to_le_bytes());
    
    let hash = hasher.finalize();
    
    let random_value = u64::from_le_bytes(hash[..8].try_into().unwrap());

    
    (random_value as usize) % (max + 1)
}

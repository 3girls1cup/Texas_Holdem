use std::collections::HashSet;
use secret_toolkit_crypto::hkdf_sha_512;
use secret_toolkit_permit::{validate, Permit};
use sha2::{Digest, Sha256};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use crate::error::{ContractError, QueryError};
use crate::msg::{CommunityCardsResponse, ExecuteMsg, InstantiateMsg, PlayerDataResponse, QueryMsg, QueryWithPermit, ResponsePayload, ShowdownResponse, StartGameResponse};
use crate::state::{ delete_table, load_table, save_table, CommunityCards, CommunityCardsWrapper, Config, Deck, GameState, Player, PlayerCards, PokerTable, CONFIG_KEY, COUNTER_KEY, PREFIX_REVOKED_PERMITS};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, StdError> {
    let contract_address = env.contract.address.clone();
    let config = Config {
        owner: info.sender,
        contract_address,
    };

    COUNTER_KEY.save(deps.storage, &init_counter(env))?;
    CONFIG_KEY.save(deps.storage, &config)?;

    Ok(Response::default())
}

fn init_counter(env: Env) -> u128 {
    let seed = env.block.random.as_ref().unwrap();

    let seed_number = u128::from_le_bytes(seed[..16].try_into().unwrap()); // Convertir les 16 premiers octets en u128

    // Extraire les 3 derniers chiffres
    let last_three_digits = seed_number % 1000; // Prend les 3 derniers chiffres décimaux

    last_three_digits as u128
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
        ExecuteMsg::StartGame {table_id, hand_ref, players, folded_win } => start_game(deps, env, table_id, hand_ref, players, folded_win),
        ExecuteMsg::CommunityCards {table_id, game_state} => distribute_community_cards(deps, table_id, game_state),
        ExecuteMsg::Showdown {table_id, all_in_showdown, show_cards} => showdown(deps, env, table_id,all_in_showdown, show_cards),
        ExecuteMsg::Random {} => random(deps, env),
    }
}

fn random(
    _deps: DepsMut,
    env: Env,
) -> Result<Response, ContractError> {
    // let mut random_numbers = Vec::new();
    let mut counter = COUNTER_KEY.load(_deps.storage)?;
    // for _ in 0..10 {
    //     let random_number = generate_random_secret(&env, &mut counter)?;
    //     random_numbers.push(random_number);
    // }

    let random_secret = generate_random_number(&env, &mut counter);
    let random_shares = additive_secret_sharing(env, 5, random_secret.unwrap(), &mut counter);

    Ok(Response::new().add_attribute_plaintext("response", format!("{:?}", random_shares)))
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
        let players = table.players.iter().find(|player| &player.public_key == pub_key);
    
        if let Some(player) = players {
            player_hands.push((player.public_key.clone(), player.decrypted_hand.hole_cards.clone()));
        } else {
            return Err(ContractError::PlayerNotFound { table_id, player: pub_key.clone() });
        }
    }

    delete_table(deps.storage, table_id)?;

    let response = ResponsePayload::Showdown(ShowdownResponse {
        table_id,
        hand_ref: table.hand_ref,
        all_in_showdown,
        players_cards: player_hands,
        community_cards: handle_all_in_showdown(&table, all_in_showdown), // Fixed borrowing
    });

    let json_response = serde_json_wasm::to_string(&response)
        .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    Ok(Response::new().add_attribute_plaintext("response", json_response))
}

fn handle_all_in_showdown(table: &PokerTable, all_in_showdown: bool) -> Option<Vec<u8>> {
    if all_in_showdown {
        match table.game_state {
            GameState::PreFlop => {
                let mut cards = table.community_cards.decrypted.flop.clone();
                cards.push(table.community_cards.decrypted.turn);
                cards.push(table.community_cards.decrypted.river);
                Some(cards)
            }
            GameState::Flop => Some(vec![table.community_cards.decrypted.turn, table.community_cards.decrypted.river]),
            GameState::Turn => Some(vec![table.community_cards.decrypted.river]),
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
        (GameState::PreFlop, GameState::Flop) => Some(table.community_cards.decrypted.flop.clone()), 
        (GameState::Flop, GameState::Turn) => Some(vec![table.community_cards.decrypted.turn]), 
        (GameState::Turn, GameState::River) => Some(vec![table.community_cards.decrypted.river]), 
        _ => return Err(ContractError::GameStateError { method: "distribute_community_cards".to_string(), table_id, needed: get_needed_game_state(game_state), actual: table.game_state.clone() }),
    };

    
    let mut updated_table = table.clone();
    let game_state_clone = game_state.clone();
    updated_table.game_state = game_state;

    save_table(deps.storage, table_id, &updated_table)?;
    
    let response = ResponsePayload::CommunityCards(CommunityCardsResponse {
        table_id,
        hand_ref: table.hand_ref,
        game_state: game_state_clone,
        community_cards: cards.unwrap(),
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
    hand_ref: u32,
    players_pubkeys: Vec<String>,
    folded_win: bool,
) -> Result<Response, ContractError> {

    let table = load_table(deps.storage, table_id);

    // If the table already exists, it means it hasn't ended yet
    if !table.is_none() && !folded_win {
        let table = table.unwrap();
        return Err(ContractError::GameStateError { method: "start_game".to_string(), table_id, needed: None, actual: table.game_state });
    }

    if players_pubkeys.len() < 2 || players_pubkeys.len() > 9 {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Number of players must be between 2 and 9")));
    }

    let unique_players: HashSet<String> = players_pubkeys.iter().map(|addr| addr.clone()).collect();
    if unique_players.len() != players_pubkeys.len() {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Duplicated public keys")));
    }

    let mut counter = COUNTER_KEY.load(deps.storage)?;

    let random_number = generate_random_number(&env, &mut counter)?;

    let mut deck = Deck::new();

    shuffle_deck(&mut deck, random_number);
    
    let mut player_cards: Vec<(String, PlayerCards)> = Vec::new();
    let mut deck_iter = deck.cards.iter();
    
    for pub_key in players_pubkeys.clone() {
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

    let mut community_cards_secrets = Vec::new();

    for _ in 0..3 {
        let secret = generate_random_number(&env, &mut counter).unwrap();
        let secret_shares = additive_secret_sharing(env.clone(), players_pubkeys.len(), secret, &mut counter);
        community_cards_secrets.push((secret, secret_shares));
    }

    let community_cards_wrapper = CommunityCardsWrapper::new(community_cards, community_cards_secrets[0].0, community_cards_secrets[1].0, community_cards_secrets[2].0);
    let mut players = Vec::new();

    for (i, (pub_key, cards)) in player_cards.iter().enumerate() {
        let encrypted_cards = encrypt_cards(random_number, cards.hole_cards.clone());
        let player = Player {
            public_key: pub_key.clone(),
            encrypted_hand: PlayerCards { hole_cards: encrypted_cards },
            decrypted_hand: PlayerCards { hole_cards: cards.hole_cards.clone() },
            hand_seed: random_number,
            flop_seed_share: community_cards_secrets[0].1[i],
            turn_seed_share: community_cards_secrets[1].1[i],
            river_seed_share: community_cards_secrets[2].1[i],
        };

        players.push(player);
    }
    let table = PokerTable {
        game_state: GameState::PreFlop,
        hand_ref,
        players,
        community_cards: community_cards_wrapper,
    };

    save_table(deps.storage, table_id, &table)?;
    COUNTER_KEY.save(deps.storage, &counter)?;
    let response = ResponsePayload::StartGame(StartGameResponse {
        table_id,
        hand_ref,
        players: players_pubkeys,
        folded_win,
    });
    let json_response = serde_json_wasm::to_string(&response)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

Ok(Response::new().add_attribute_plaintext("response", json_response))
}

// fn generate_random_number(
//     env: &Env,
//     counter: u32,
// ) -> StdResult<u64> {
//     let seed = env.block.random.as_ref().unwrap();
//     let mut hasher = Sha256::new();
//     hasher.update(seed.as_slice());
//     hasher.update(&counter.to_le_bytes());
//     let mut okm = [0u8; 8];
//     let derived_seed = hasher.finalize();
//     let hk = hkdf::Hkdf::<Sha256>::new(None, &derived_seed);
//     hk.expand(b"random_number", &mut okm).map_err(|_| StdError::generic_err("HKDF expand failed"))?;
//     let final_seed = u64::from_le_bytes(okm);

//     Ok(final_seed)
// }

fn generate_random_number(env: &Env, counter: &mut u128) -> StdResult<u64> {
    let secret = hkdf_sha_512(
        &Some(vec![0u8; 64]),
        &env.block.random.as_ref().unwrap(),
        &counter.to_le_bytes(),
        64,
    )?;

    *counter += 1;

    Ok(u64::from_le_bytes(secret[..8].try_into().unwrap()))
}

pub fn encrypt_cards<T>(secret: u64, cards: T) -> T
where
    T: Encryptable,
{
    cards.encrypt(secret)
}

/// Définition du Trait pour gérer les types `u8` et `Vec<u8>`
pub trait Encryptable {
    fn encrypt(self, secret: u64) -> Self;
}

/// Implémentation pour `u8`
impl Encryptable for u8 {
    fn encrypt(self, secret: u64) -> Self {
        self.wrapping_add(secret as u8)
    }
}

/// Implémentation pour `Vec<u8>`
impl Encryptable for Vec<u8> {
    fn encrypt(mut self, secret: u64) -> Self {
        self.iter_mut().for_each(|card| *card = card.wrapping_add(secret as u8));
        self
    }
}


fn decrypt_cards(secret: u64, cards: Vec<u8>) -> Vec<u8> {
    let mut decrypted_cards = Vec::new();
    for card in cards {
        decrypted_cards.push(card.wrapping_sub(secret as u8));
    }

    decrypted_cards
}


fn additive_secret_sharing(env: Env, players: usize, secret: u64, counter: &mut u128) -> Vec<u64> {
    let mut shares: Vec<u64> = Vec::new();
    let mut sum: u64 = 0;

    for _ in 0..(players - 1) {
        let share = generate_random_number(&env, counter).unwrap();
        shares.push(share);
        sum = sum.wrapping_add(share);
    }

    let last_share = secret.wrapping_sub(sum); // Assurer que la somme = secret
    shares.push(last_share);

    shares
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{attr, testing::{mock_dependencies, mock_env, mock_info}};

    use super::*;


    #[test]
    fn test_random() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {};
        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Random {};
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes.len(), 1);
        assert!(res.attributes[0].key == "response");
        println!("{:?}", res.attributes[0].value);
    }

    #[test]
    fn test_init_counter() {
        let env = mock_env();
        let counter = init_counter(env);
        println!("{:?}", counter);
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {};
        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        assert_eq!(res.attributes, vec![attr("method", "instantiate")]);

        let config = CONFIG_KEY.load(&deps.storage).unwrap();
        assert_eq!(config.owner, info.sender);
        assert_eq!(config.contract_address, env.contract.address);
    }

    #[test]
    fn test_start_game() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {};
        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let players = vec!["player1".to_string(), "player2".to_string()];
        let msg = ExecuteMsg::StartGame {
            table_id: 1,
            hand_ref: 1,
            players: players.clone(),
            folded_win: false,
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("method", "start_game")]);

        let table = load_table(&deps.storage, 1).unwrap();
        assert_eq!(table.hand_ref, 1);
        assert_eq!(table.players.len(), players.len());
    }

    #[test]
    fn test_distribute_community_cards() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {};
        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let players = vec!["player1".to_string(), "player2".to_string()];
        let start_msg = ExecuteMsg::StartGame {
            table_id: 1,
            hand_ref: 1,
            players: players.clone(),
            folded_win: false,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), start_msg).unwrap();

        let msg = ExecuteMsg::CommunityCards {
            table_id: 1,
            game_state: GameState::Flop,
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("method", "distribute_community_cards")]);

        let table = load_table(&deps.storage, 1).unwrap();
        assert_eq!(table.game_state, GameState::Flop);
    }

    #[test]
    fn test_showdown() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {};
        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let players = vec!["player1".to_string(), "player2".to_string()];
        let start_msg = ExecuteMsg::StartGame {
            table_id: 1,
            hand_ref: 1,
            players: players.clone(),
            folded_win: false,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), start_msg).unwrap();

        let msg = ExecuteMsg::Showdown {
            table_id: 1,
            all_in_showdown: true,
            show_cards: players.clone(),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes, vec![attr("method", "showdown")]);

        let table = load_table(&deps.storage, 1);
        assert!(table.is_none());
    }

  
}

// fn generate_random_number_(
//     env: &Env,
// ) -> StdResult<u64> {
//     let seed = env.block.random.as_ref().unwrap();
//     let mut hasher = Sha256::new();

//     hasher.update(seed.as_slice());
    
//     let final_hash = hasher.finalize();
//     let final_seed = u64::from_le_bytes(final_hash[..8].try_into().unwrap());

//     Ok(final_seed)
// }


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
        QueryMsg::CommunityCards { table_id, game_state, secret_key } => to_binary(&query_community_cards(deps, table_id, game_state, secret_key)),
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
        QueryWithPermit::PlayerPrivateData {table_id} => {
            to_binary(&query_player_private_data(deps, table_id, viewer))
        },
    }
}

fn query_player_private_data(deps: Deps, table_id: u32, pub_key: String) -> Result<PlayerDataResponse, QueryError> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Err(QueryError::TableNotFound { table_id });
    }

    let table = table.unwrap();
    let player = table.players.iter().find(|player| &player.public_key == &pub_key);

    if player.is_none() {
        return Err(QueryError::PlayerNotFound { table_id, player: pub_key });
    }

    let player = player.unwrap();

    Ok(PlayerDataResponse {
        table_id,
        hand_ref: table.hand_ref,
        hand: player.decrypted_hand.hole_cards.clone(),
        hand_seed: player.hand_seed,
        flop_secret: player.flop_seed_share,
        turn_secret: player.turn_seed_share,
        river_secret: player.river_seed_share,
    })


}

fn query_community_cards(deps: Deps, table_id: u32, game_state: GameState, secret_key: u64) -> Result<CommunityCardsResponse, QueryError> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Err(QueryError::TableNotFound { table_id });
    }

    let table = table.unwrap();
    let (stored_key, cards) = match game_state {
        GameState::Flop => (table.community_cards.flop_secret, table.community_cards.decrypted.flop),
        GameState::Turn => (table.community_cards.turn_secret, vec![table.community_cards.decrypted.turn]),
        GameState::River => (table.community_cards.river_secret, vec![table.community_cards.decrypted.river]),
        _ => return Err(QueryError::InvalidGameState { game_state }),
    };

    if stored_key != secret_key {
        return Err(QueryError::InvalidViewingKey { key: secret_key } );
    }

    Ok(CommunityCardsResponse {
        table_id,
        hand_ref: table.hand_ref,
        game_state: table.game_state,
        community_cards: cards,
    })
}

fn query_showdown(deps: Deps, table_id: u32, flop_secret: Option<u64>, turn_secret: Option<u64>,river_secret: Option<u64>, players_secrets: Vec<u64>,) -> Result<ShowdownResponse, QueryError> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Err(QueryError::TableNotFound { table_id });
    }
    let table = table.unwrap();
    let mut community_cards = Vec::new();

    if let Some(secret) = flop_secret {
        if table.community_cards.flop_secret != secret {
            return Err(QueryError::InvalidViewingKey { key: secret });
        }
        community_cards.extend(table.community_cards.decrypted.flop.clone());
    }

    if let Some(secret) = turn_secret {
        if table.community_cards.turn_secret != secret {
            return Err(QueryError::InvalidViewingKey { key: secret });
        }
        community_cards.push(table.community_cards.decrypted.turn);
    }

    if let Some(secret) = river_secret {
        if table.community_cards.river_secret != secret {
            return Err(QueryError::InvalidViewingKey { key: secret });
        }
        community_cards.push(table.community_cards.decrypted.river);
    }

    let mut players_cards = Vec::new();

    for (i, secret) in players_secrets.iter().enumerate() {
        let player = table.players.iter().find(|player| &player.hand_seed == secret).ok_or_else(|| QueryError::PlayerNotFound { table_id, player: i.to_string() })?;
        
        players_cards.push((player.public_key.clone(), player.decrypted_hand.hole_cards.clone()));
    }
}

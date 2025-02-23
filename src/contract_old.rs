use std::collections::HashSet;
use secret_toolkit_crypto::hkdf_sha_512;
use secret_toolkit_permit::{validate, Permit};
use sha2::{Digest, Sha256};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult
};
use uuid::Uuid;
use crate::error::{ContractError, QueryError};
use crate::msg::{CommunityCardsResponse, ExecuteMsg, LastHandLogResponse, QueryMsg, QueryWithPermit, ResponsePayload, ShowdownPlayer, ShowdownResponse, StartGamePlayer, StartGameResponse};
use crate::state::{ delete_table, load_table, save_table, Card, CommunityCards, Config, Deck, Flop, GameState, Player, PokerTable, River, Turn, CONFIG_KEY, COUNTER_KEY, PREFIX_REVOKED_PERMITS};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
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
        ExecuteMsg::StartGame {table_id, hand_ref, players , prev_hand_showdown_players} => start_game(deps, env, table_id, hand_ref, players, prev_hand_showdown_players),
        ExecuteMsg::CommunityCards {table_id, game_state} => distribute_community_cards(deps, env, table_id, game_state),
        ExecuteMsg::Showdown {table_id, game_state, show_cards} => showdown(deps, env, table_id, game_state, show_cards),
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
    env: Env,
    table_id: u32,
    game_state: GameState,
    show_cards: Vec<String>,
) -> Result<Response, ContractError> {
    let mut table = load_table(deps.storage, table_id)
        .ok_or_else(|| ContractError::TableNotFound { table_id })?;

    let mut player_hands: Vec<(Uuid, Vec<Card>)> = Vec::new();

    for pub_key in show_cards.iter() {
        let players = table.players.iter().find(|player| &player.public_key == pub_key);
    
        if let Some(player) = players {
            player_hands.push((player.player_id.clone(), player.hand.clone()));
        } else {
            return Err(ContractError::PlayerNotFound { table_id, player: pub_key.clone() });
        }
    }

    delete_table(deps.storage, table_id)?;

    let response = ResponsePayload::Showdown(ShowdownResponse {
        table_id,
        hand_ref: table.hand_ref,
        players_cards: player_hands,
        community_cards: handle_all_in_showdown(&table.community_cards, game_state),
    });

    // Log the showdown retrieval time
    table.showdown_retrieved_at = Some(env.block.time);
    save_table(deps.storage, table_id, &table)?;

    let json_response = serde_json_wasm::to_string(&response)
        .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    Ok(Response::new().add_attribute_plaintext("response", json_response))
}

fn handle_all_in_showdown(community_cards: &CommunityCards, game_state: GameState) -> Option<Vec<Card>> {
    match game_state {
        GameState::PreFlop => {
            let mut cards = community_cards.flop.cards.clone();
            cards.push(community_cards.turn.card.clone());
            cards.push(community_cards.river.card.clone());
            Some(cards)
        }
        GameState::Flop => Some(vec![community_cards.turn.card.clone(), community_cards.river.card.clone()]),
        GameState::Turn => Some(vec![community_cards.river.card.clone()]),
        _ => return None,
    }
}

fn distribute_community_cards(
    deps: DepsMut,
    env: Env,
    table_id: u32,
    game_state: GameState,
) -> Result<Response, ContractError> {

    let mut table = load_table(deps.storage, table_id)
        .ok_or_else(|| ContractError::TableNotFound { table_id })?;
    let cards = match game_state {
        GameState::Flop => {
            table.community_cards.flop.retrieved_at = Some(env.block.time);
            Some(table.community_cards.flop.cards.clone())
        }, 
        GameState::Turn => {
            table.community_cards.turn.retrieved_at = Some(env.block.time);
            Some(vec![table.community_cards.turn.card.clone()])
        }, 
        GameState::River=> {
            table.community_cards.river.retrieved_at = Some(env.block.time);
            Some(vec![table.community_cards.river.card.clone()])
        }, 
        _ => return Err(ContractError::GameStateError { method: "distribute_community_cards".to_string(), table_id, game_state: Some(game_state) }),
    };

    // Log the retrieved_at time
    save_table(deps.storage, table_id, &table)?;
    
    let response = ResponsePayload::CommunityCards(CommunityCardsResponse {
        table_id,
        hand_ref: table.hand_ref,
        game_state: game_state,
        community_cards: cards.unwrap(),
    });

    let json_response = serde_json_wasm::to_string(&response)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    Ok(Response::new()
        .add_attribute_plaintext("response", json_response))
}

fn start_game(
    deps: DepsMut,
    env: Env,
    table_id: u32,
    hand_ref: u32,
    players_info: Vec<StartGamePlayer>,
    prev_hand_showdown_players: Vec<Uuid>,
) -> Result<Response, ContractError> {

    let table = load_table(deps.storage, table_id);

    let previous_hand_log = if table.is_some() {
        let table = table.unwrap();
        
        Some(LastHandLogResponse {
            showdown_players: prev_hand_showdown_players.iter().map(|player_id| {
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

    if players_info.len() < 2 || players_info.len() > 9 {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Number of players must be between 2 and 9")));
    }

    let unique_players: HashSet<String> = players_info.iter().map(|player| player.public_key.clone()).collect();
    
    if unique_players.len() != players_info.len() {
        return Err(ContractError::from(cosmwasm_std::StdError::generic_err("Duplicated public keys")));
    }

    let mut counter = COUNTER_KEY.load(deps.storage)?;

    let random_number = generate_random_number(&env, &mut counter)?;

    let mut deck = Deck::new();

    shuffle_deck(&mut deck, random_number);
    
    let mut player_cards: Vec<(String, Vec<Card>)> = Vec::new();
    let mut deck_iter = deck.cards.iter();
    
    for i in 0..players_info.len() {
        let cards = vec![deck_iter.next().unwrap().clone(), deck_iter.next().unwrap().clone()];
        player_cards.push((players_info[i].public_key.clone(), cards.clone()));
    }

    let mut community_cards_secrets = Vec::new();

    for _ in 0..3 {
        let secret = generate_random_number(&env, &mut counter).unwrap();
        let secret_shares = additive_secret_sharing(env.clone(), players_info.len(), secret, &mut counter);
        community_cards_secrets.push((secret, secret_shares));
    }

    let community_cards = CommunityCards {
        flop: Flop {
            cards: vec![deck_iter.next().unwrap().clone(), deck_iter.next().unwrap().clone(), deck_iter.next().unwrap().clone()],
            secret: community_cards_secrets[0].0,
            retrieved_at: None,
        },
        turn: Turn {
            card: deck_iter.next().unwrap().clone(),
            secret: community_cards_secrets[1].0,
            retrieved_at: None,

        },
        river: River {
            card: deck_iter.next().unwrap().clone(),
            secret: community_cards_secrets[2].0,
            retrieved_at: None,
        },
    };

    let mut players = Vec::new();

    for (i, (pub_key, cards)) in player_cards.iter().enumerate() {
        let player = Player {
            username: players_info[i].username.clone(),
            player_id: players_info[i].player_id,
            public_key: pub_key.clone(),
            hand: cards.clone(),
            hand_secret: generate_random_number(&env, &mut counter)?,
            flop_secret_share: community_cards_secrets[0].1[i],
            turn_secret_share: community_cards_secrets[1].1[i],
            river_secret_share: community_cards_secrets[2].1[i],
        };

        players.push(player);
    }

    let table = PokerTable {
        hand_ref,
        players: players.clone(),
        community_cards,
        showdown_retrieved_at: None,
    };

    save_table(deps.storage, table_id, &table)?;
    COUNTER_KEY.save(deps.storage, &counter)?;
    let response = ResponsePayload::StartGame(StartGameResponse {
        table_id,
        hand_ref,
        players: players.iter().map(|player| player.username.clone()).collect(),
    });

    let response = serde_json_wasm::to_string(&response)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

    let previous_hand_log = serde_json_wasm::to_string(&previous_hand_log)
    .map_err(|e| cosmwasm_std::StdError::generic_err(format!("Serialization error: {}", e)))?;

Ok(Response::new().add_attribute_plaintext("response", response).add_attribute_plaintext("previous_hand", previous_hand_log))
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
        QueryMsg::Showdown { table_id, flop_secret, turn_secret, river_secret, players_secrets } => to_binary(&query_showdown(deps, table_id, flop_secret, turn_secret, river_secret, players_secrets)),
    }
}



fn test_permit_queries(
    deps: Deps,
    pubkey: String,
    query: QueryWithPermit,
) -> StdResult<Binary> {
    // Validate permit content

    // Permit validated! We can now execute the query.
    match query {
        QueryWithPermit::PlayerPrivateData {table_id} => {
            to_binary(&query_player_private_data(deps, table_id, pubkey))
        },
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

fn query_player_private_data(deps: Deps, table_id: u32, pub_key: String) -> Result<Player, QueryError> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Err(QueryError::TableNotFound { table_id });
    }

    let table = table.unwrap();
    let player = table.players.iter().find(|player| &player.public_key == &pub_key);

    if player.is_none() {
        return Err(QueryError::PlayerNotFound { table_id, player: pub_key });
    }

    Ok(player.unwrap().clone())
}

fn query_community_cards(deps: Deps, table_id: u32, game_state: GameState, secret_key: u64) -> Result<CommunityCardsResponse, QueryError> {
    let table = load_table(deps.storage, table_id);

    if table.is_none() {
        return Err(QueryError::TableNotFound { table_id });
    }

    let table = table.unwrap();
    let (stored_key, cards) = match game_state {
        GameState::Flop => (table.community_cards.flop.secret, table.community_cards.flop.cards),
        GameState::Turn => (table.community_cards.turn.secret, vec![table.community_cards.turn.card]),
        GameState::River => (table.community_cards.river.secret, vec![table.community_cards.river.card]),
        _ => return Err(QueryError::InvalidGameState { game_state }),
    };

    if stored_key != secret_key {
        return Err(QueryError::InvalidViewingKey { key: secret_key } );
    }

    Ok(CommunityCardsResponse {
        table_id,
        hand_ref: table.hand_ref,
        game_state,
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

    let mut players_cards = Vec::new();

    for secret in players_secrets.iter() {
        let player = table.players.iter().find(|player| &player.hand_secret == secret).ok_or_else(|| QueryError::SecretNotFound { val: secret.to_string() })?;
        
        players_cards.push((player.player_id.clone(), player.hand.clone()));
    }

    Ok(ShowdownResponse {
        table_id,
        hand_ref: table.hand_ref,
        players_cards,
        community_cards: Some(community_cards),
    })
}

#[cfg(test)]
mod tests {

use std::str::FromStr;

use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

use super::*;


    #[test]
    fn test_random() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), env.clone(), info.clone()).unwrap();

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
    fn start_game() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        instantiate(deps.as_mut(), env.clone(), info.clone()).unwrap();

        let players = vec![
            StartGamePlayer {
                username: "player1".to_string(),
                player_id: Uuid::from_str("54d8f23e-3e5e-4462-910c-fb36079f6c31").unwrap(),
                public_key: "public_key1".to_string(),
            },
            StartGamePlayer {
                username: "player2".to_string(),
                player_id: Uuid::from_str("955f039a-ab05-49f3-83a9-720980cf3832").unwrap(),
                public_key: "public_key2".to_string(),
            },
        ];

        let msg = ExecuteMsg::StartGame {
            table_id: 1,
            hand_ref: 1,
            players,
            prev_hand_showdown_players: vec![],
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(res.attributes.len(), 2);
        assert!(res.attributes[0].key == "response");
        println!("{:?}", res.attributes[0].value);

        let query = QueryWithPermit::PlayerPrivateData { table_id: 1 };

        let res = test_permit_queries(deps.as_ref(), "public_key3".to_string(), query);
        println!("{:?}", String::from_utf8(res.unwrap().to_vec()).unwrap());

    }

    #[test]
fn test_query_showdown() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Initialiser le contrat
    instantiate(deps.as_mut(), env.clone(), info.clone()).unwrap();

    // Créer une table de jeu
    let players = vec![
        StartGamePlayer {
            username: "player1".to_string(),
            player_id: Uuid::from_str("54d8f23e-3e5e-4462-910c-fb36079f6c31").unwrap(),
            public_key: "public_key1".to_string(),
        },
        StartGamePlayer {
            username: "player2".to_string(),
            player_id: Uuid::from_str("955f039a-ab05-49f3-83a9-720980cf3832").unwrap(),
            public_key: "public_key2".to_string(),
        },
    ];

    let msg = ExecuteMsg::StartGame {
        table_id: 1,
        hand_ref: 1,
        players: players.clone(),
        prev_hand_showdown_players: vec![],
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Distribuer les cartes communes (flop, turn, river)
    let msg = ExecuteMsg::CommunityCards {
        table_id: 1,
        game_state: GameState::Flop,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::CommunityCards {
        table_id: 1,
        game_state: GameState::Turn,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    let msg = ExecuteMsg::CommunityCards {
        table_id: 1,
        game_state: GameState::River,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Récupérer les cartes lors du showdown
    let table = load_table(deps.as_ref().storage, 1).unwrap();
    let flop_secret = Some(table.community_cards.flop.secret);
    let turn_secret = Some(table.community_cards.turn.secret);
    let river_secret = Some(table.community_cards.river.secret);
    let players_secrets = [table.players.iter().map(|player| player.hand_secret).collect(), 
    vec![]].concat();

    let msg = QueryMsg::Showdown {
        table_id: 1,
        flop_secret,
        turn_secret,
        river_secret,
        players_secrets,
    };
    let res = query(deps.as_ref(), env.clone(), msg);

    // Vérifier que la réponse contient les bonnes informations
    let showdown = String::from_utf8(res.unwrap().to_vec());
    println!("{:?}", showdown.unwrap());
}

#[test]
fn test_query_community_cards() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("creator", &[]);

    // Initialiser le contrat
    instantiate(deps.as_mut(), env.clone(), info.clone()).unwrap();

    // Créer une table de jeu
    let players = vec![
        StartGamePlayer {
            username: "player1".to_string(),
            player_id: Uuid::from_str("54d8f23e-3e5e-4462-910c-fb36079f6c31").unwrap(),
            public_key: "public_key1".to_string(),
        },
        StartGamePlayer {
            username: "player2".to_string(),
            player_id: Uuid::from_str("955f039a-ab05-49f3-83a9-720980cf3832").unwrap(),
            public_key: "public_key2".to_string(),
        },
    ];

    let msg = ExecuteMsg::StartGame {
        table_id: 1,
        hand_ref: 1,
        players: players.clone(),
        prev_hand_showdown_players: vec![],
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Distribuer les cartes communes (flop)
    let msg = ExecuteMsg::CommunityCards {
        table_id: 1,
        game_state: GameState::Flop,
    };
    execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

    // Récupérer les cartes communes avec une clé secrète
    let table = load_table(deps.as_ref().storage, 1).unwrap();
    let secret_key = table.community_cards.flop.secret;

    let msg = QueryMsg::CommunityCards {
        table_id: 1,
        game_state: GameState::Flop,
        secret_key,
    };
    let res = query(deps.as_ref(), env.clone(), msg);

    // Vérifier que la réponse contient les bonnes informations
    let community_cards = String::from_utf8(res.unwrap().to_vec());
    println!("{:?}", community_cards.unwrap());
}
}
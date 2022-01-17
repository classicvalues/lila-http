use serde::{Deserialize, Serialize};

use serde_json::Value as JsValue;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Eq, PartialEq, Deserialize, Hash, Clone)]
pub struct ArenaId(pub String);

// naming is hard
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaShared {
    nb_players: u32,
    duels: JsValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seconds_to_finish: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seconds_to_start: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_started: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_finished: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    is_recently_finished: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    featured: Option<JsValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    podium: Option<JsValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pairings_closed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stats: Option<JsValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    team_standing: Option<JsValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duel_teams: Option<JsValue>,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
pub struct UserId(pub String);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserName(pub String);
impl UserName {
    pub fn to_id(&self) -> UserId {
        UserId(self.0.to_lowercase())
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameId(String);
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rank(pub usize);

pub struct ArenaFull {
    pub id: ArenaId,
    pub shared: Arc<ArenaShared>,
    pub ongoing_user_games: HashMap<UserId, GameId>,
    pub standing: Vec<Player>,
    pub ranking: FullRanking,
}

#[derive(Debug, Clone)]
pub struct FullRanking(pub HashMap<UserId, Rank>);

#[derive(Debug, Clone, Serialize)]
struct ClientMe {
    rank: Option<Rank>,
    withdraw: bool,
    game_id: Option<GameId>,
    pause_delay: Option<u32>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Player {
    pub name: UserName,
    #[serde(flatten)]
    rest: JsValue,
}

#[derive(Debug, Clone, Serialize)]
struct ClientStanding {
    page: u32,
    players: Vec<Player>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientData {
    #[serde(flatten)]
    shared: Arc<ArenaShared>,
    #[serde(skip_serializing_if = "Option::is_none")]
    me: Option<ClientMe>,
    standing: ClientStanding,
}

impl ClientData {
    pub fn new(full: Arc<ArenaFull>, user_id: Option<UserId>) -> ClientData {
        let page = 1;
        let players = full.standing.chunks(10).nth(page - 1).unwrap_or_default();
        ClientData {
            shared: Arc::clone(&full.shared),
            me: user_id.map(|uid| {
                ClientMe {
                    rank: full.ranking.0.get(&uid).cloned(),
                    withdraw: false, // todo!(),
                    game_id: full.ongoing_user_games.get(&uid).cloned(),
                    pause_delay: None,
                }
            }),
            standing: ClientStanding {
                page: 1,
                players: players.to_vec(),
            },
        }
    }
}

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use futures::stream::StreamExt;
use redis::RedisError;
use serde::Deserialize;
use serde_json::{Error as SerdeJsonError, Value as JsValue};
use serde_with::{serde_as, DisplayFromStr};

use crate::{
    arena::{
        ArenaFull, ArenaId, ArenaShared, OngoingUserGames, PauseSeconds, Player, PlayerMapEntry,
        Rank, Sheet, SheetScores, TeamId, TeamStanding, UserId, UserName,
    },
    opt::RedisOpt,
    repo::Repo,
};

fn parse_message(msg: &redis::Msg) -> Result<ArenaFullRedis, SerdeJsonError> {
    serde_json::from_slice(msg.get_payload_bytes())
}

pub async fn subscribe(opt: RedisOpt, repo: &'static Repo) {
    let client = redis::Client::open(opt.redis_url).expect("valid redis url");
    loop {
        if let Err(err) = subscribe_inner(&client, repo).await {
            log::error!("{}", err);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn subscribe_inner(client: &redis::Client, repo: &'static Repo) -> Result<(), RedisError> {
    log::info!("Redis stream connecting ...");
    let con = client.get_tokio_connection().await?;
    let mut pubsub = con.into_pubsub();
    pubsub.subscribe("http-out").await?;
    let mut stream = pubsub.on_message();
    log::info!("Redis stream connected.");

    while let Some(msg) = stream.next().await {
        match parse_message(&msg) {
            Ok(full) => repo.put(full.expand()).await,
            Err(msg) => log::error!("Failed to parse message: {}", msg),
        }
    }
    log::error!("Redis stream end.");

    Ok(())
}

#[derive(Deserialize, Clone, Debug)]
struct PlayerRedis {
    pub name: UserName,
    #[serde(default)]
    pub withdraw: bool,
    pub sheet: SheetScores,
    #[serde(default)]
    pub fire: bool,
    pub team: Option<TeamId>,
    pub pause: Option<PauseSeconds>,
    #[serde(flatten)]
    rest: JsValue,
}

impl PlayerRedis {
    fn expand(self, rank: Rank) -> Player {
        Player {
            name: self.name,
            withdraw: self.withdraw,
            sheet: Sheet {
                fire: self.fire,
                scores: self.sheet,
            },
            rank,
            team: self.team,
            rest: self.rest,
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArenaFullRedis {
    pub id: ArenaId,
    #[serde(flatten)]
    pub shared: ArenaShared,
    #[serde_as(as = "DisplayFromStr")]
    pub ongoing_user_games: OngoingUserGames,
    pub standing: Vec<PlayerRedis>,
    pub team_standing: Option<TeamStanding>,
}

impl ArenaFullRedis {
    pub fn expand(self) -> ArenaFull {
        let withdrawn = standing_to_withdrawn(&self.standing);
        let pauses = standing_to_pauses(&self.standing);
        let player_vec: Vec<Player> = self
            .standing
            .into_iter()
            .enumerate()
            .map(|(index, player)| player.expand(Rank(index + 1)))
            .collect();
        ArenaFull {
            id: self.id,
            ongoing_user_games: self.ongoing_user_games,
            withdrawn,
            player_map: make_player_map(&player_vec),
            player_vec,
            team_standing: self.team_standing,
            shared: self.shared,
            pauses,
        }
    }
}

fn make_player_map(standing: &[Player]) -> HashMap<UserId, PlayerMapEntry> {
    standing
        .iter()
        .map(|player| {
            (
                player.name.clone().into_id(),
                PlayerMapEntry {
                    rank: player.rank,
                    team: player.team.clone(),
                },
            )
        })
        .collect()
}

fn standing_to_withdrawn(standing: &[PlayerRedis]) -> HashSet<UserId> {
    standing
        .iter()
        .filter(|p| p.withdraw)
        .map(|p| p.name.clone().into_id())
        .collect()
}

fn standing_to_pauses(standing: &[PlayerRedis]) -> HashMap<UserId, PauseSeconds> {
    standing
        .iter()
        .filter_map(|player| {
            player
                .pause
                .map(|pause| (player.name.clone().into_id(), pause))
        })
        .collect()
}

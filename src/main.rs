use anyhow::{anyhow, Result as AnyhowResult};
use num_cpus;
use reqwest::{self, Client};
use rusqlite::{Connection, Result as RSqlResult};
use scraper::{Html, Selector};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use tokio::{self};

const TARGET_URL: &str = "http://rwr.runningwithrifles.com/rwr_stats/view_players.php";
const SELECTOR_MATCH: &str = "table > tbody > tr";
const PAGE_SIZE: u8 = 100;
const DB_NAME: &str = "rwr_players.db";
const TABLE_NAME: &str = "rwr_players";

#[derive(Debug)]
struct Player {
    username: String,
    kills: i128,
    deaths: i128,
    score: i128,
    // N minutes count
    time_played: i128,
    longest_kill_streak: i128,
    targets_destroyed: i128,
    soldiers_healed: i128,
    teamkills: i128,
    // x.y km
    distance_moved: f64,
    shots_fired: i128,
    throwables_thrown: i128,
    // XP
    rank_progression: i128,
    // Private / True rank name
    rank_name: String,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            username: "".to_string(),
            kills: 0,
            deaths: 0,
            score: 0,
            time_played: 0,
            longest_kill_streak: 0,
            targets_destroyed: 0,
            soldiers_healed: 0,
            teamkills: 0,
            distance_moved: 0.0,
            shots_fired: 0,
            throwables_thrown: 0,
            rank_progression: 0,
            rank_name: "".to_string(),
        }
    }
}

fn get_drop_table_sql(table_name: &str) -> String {
    format!("DROP TABLE IF EXISTS {}", table_name)
}

fn get_create_table_sql(table_name: &str) -> String {
    format!(
        "CREATE TABLE \"{}\" (
	\"id\" INTEGER NOT NULL,
	\"username\" TEXT NOT NULL,
	\"kills\" BIGINT NOT NULL,
	\"deaths\" BIGINT NOT NULL,
	\"score\" BIGINT NOT NULL,
	\"time_played\" BIGINT NOT NULL,
	\"longest_kill_streak\" BIGINT NOT NULL,
	\"targets_destroyed\" BIGINT NOT NULL,
	\"soldiers_healed\" BIGINT NOT NULL,
	\"teamkills\" BIGINT NOT NULL,
	\"distance_moved\" REAL NOT NULL,
	\"shots_fired\" BIGINT NOT NULL,
	\"throwables_thrown\" BIGINT NOT NULL,
	\"rank_progression\" BIGINT NOT NULL,
	\"rank_name\" TEXT NOT NULL,
	PRIMARY KEY (\"id\")
);",
        table_name
    )
}

fn quick_selector(exp: &str) -> Selector {
    Selector::parse(exp).unwrap()
}

fn insert_player_data(conn: &Connection, player: Player) -> RSqlResult<usize> {
    let sql_text = format!(
        "INSERT INTO rwr_players (username, kills, deaths, score,time_played,longest_kill_streak,targets_destroyed,soldiers_healed,teamkills,distance_moved,shots_fired,throwables_thrown,rank_progression,rank_name)
VALUES('{}',{},{},{},{},{},{},{},{},{},{},{},{},'{}')",
                           player.username,
                           player.kills,
                           player.deaths,
                           player.score,
                           player.time_played,
                           player.longest_kill_streak,
                           player.targets_destroyed,
                           player.soldiers_healed,
                           player.teamkills,
                           player.distance_moved,
                           player.shots_fired,
                           player.throwables_thrown,
                           player.rank_progression,
                           player.rank_name
    );

    conn.execute(&sql_text, ())
}

async fn run_task(client: Arc<Client>, conn: Arc<Mutex<Connection>>, start: Arc<Mutex<i128>>) -> AnyhowResult<()> {
    loop {
        // 先改值, 使得下一次数据正确
        let current_start = {
            let mut next_start = start.lock().unwrap();
            let start = *next_start;
            *next_start += PAGE_SIZE as i128;
            start
        };

        let resp = client
            .get(TARGET_URL)
            .query(&[
                ("db", "invasion"),
                ("sort", "rank_progression"),
                ("start", &current_start.to_string()),
            ])
            .send()
            .await?
            .text()
            .await?;

        let fragment = Html::parse_fragment(&resp);
        let selector = quick_selector(SELECTOR_MATCH);

        let mut property_map: Vec<String> = vec![];

        let mut data_size: i128 = -1;

        for element in fragment.select(&selector) {
            println!(
                ">>>>>Start Parsing... start:{}, data(before):{}>>>>>",
                current_start, data_size
            );
            // println!("tr element: {:?}", element.value());

            // column name
            for th in element.select(&quick_selector("th")) {
                // println!("th element: {:?}", th.value());

                for div in th.select(&quick_selector("div")) {
                    let property_name = div.value().classes().into_iter().next().unwrap();
                    println!("Parsing... column head: {}", property_name);

                    // println!("div element class: {}", property_name);

                    property_map.push(property_name.to_string());
                }
            }

            let mut player = Player::default();

            // data
            for (index, td) in element.select(&quick_selector("td")).enumerate() {
                match td.text().next() {
                    Some(t) => {
                        let key = property_map.iter().nth(index);

                        // println!("data: {:?}: {}", k, t);

                        if let Some(k) = key {
                            println!("data: {:?}: {}", k, t);
                            match k.as_str() {
                                "username" => {
                                    player.username = String::from(t);
                                    println!("username: {}", t);
                                }
                                "kills" => {
                                    player.kills = t.parse()?;
                                    println!("kills: {}", t);
                                }
                                "deaths" => {
                                    player.deaths = t.parse()?;
                                    println!("deaths: {}", t);
                                }
                                "score" => {
                                    player.score = t.parse()?;
                                    println!("score: {}", t);
                                }
                                "time_played" => {
                                    // Example source str: 1718h 48min
                                    let times_str = t.replace("h ", "|").replace("min", "");
                                    let times_str_split_collect = times_str
                                        .split("|")
                                        .map(|s| s.to_string())
                                        .collect::<Vec<String>>();
                                    let times_str_iter =
                                        times_str_split_collect.iter().rev();

                                    let mut times_by_minute = 0;

                                    for (index, time_item) in times_str_iter.enumerate() {
                                        let v = time_item.parse::<i128>()?;
                                        if index == 0 {
                                            times_by_minute = times_by_minute + v;
                                        } else if index == 1 {
                                            times_by_minute = times_by_minute + v * 60;
                                        }
                                    }

                                    player.time_played = times_by_minute;
                                    println!("time_played: {}", t);
                                }
                                "longest_kill_streak" => {
                                    player.longest_kill_streak = t.parse()?;
                                    println!("longest_kill_streak: {}", t);
                                }
                                "targets_destroyed" => {
                                    player.targets_destroyed = t.parse()?;
                                    println!("targets_destroyed: {}", t);
                                }
                                "soldiers_healed" => {
                                    player.soldiers_healed = t.parse()?;
                                    println!("soldiers_healed: {}", t);
                                }
                                "teamkills" => {
                                    player.teamkills = t.parse()?;
                                    println!("teamkills: {}", t);
                                }
                                "distance_moved" => {
                                    let distance_str = t.replace("km", "");
                                    player.distance_moved = distance_str.parse()?;
                                    println!("distance_moved: {}", t);
                                }
                                "shots_fired" => {
                                    player.shots_fired = t.parse()?;
                                    println!("shots_fired: {}", t);
                                }
                                "throwables_thrown" => {
                                    player.throwables_thrown = t.parse()?;
                                    println!("throwables_thrown: {}", t);
                                }
                                "rank_progression" => {
                                    player.rank_progression = t.parse()?;
                                    println!("rank_progression: {}", t);
                                }
                                "rank_name" => {
                                    player.rank_name = String::from(t);
                                    println!("rank_name: {}", t);
                                }
                                _ => {
                                    println!("Not Found match: {}", t);
                                }
                            }
                        }
                    }
                    _ => {
                        // img, ignore it
                    }
                }
            }

            if data_size != -1 {
                println!("Player Parse:");
                println!("{:?}", player);

                let conn = &*conn.lock().unwrap();
                insert_player_data(&conn, player)?;
            }

            data_size = data_size + 1;
        }

        println!(
            ">>>>>Parsing completed, start:{}, data(after):{}>>>>>",
            current_start, data_size
        );

        if data_size < PAGE_SIZE.into() {
            println!("=====Parsing End=====");
            if data_size != -1 {
                println!("=====Total data: {}=====", current_start - (PAGE_SIZE as i128));
            } else {
                println!("=====Total data: {}=====", current_start - (PAGE_SIZE as i128) + data_size);
            }
            return Ok(());
        }
    }
}


#[tokio::main]
async fn main() -> AnyhowResult<()> {
    println!("Creating SQLite connection...");
    let origin_conn = Connection::open(DB_NAME)?;


    println!("Dropping SQLite Table...");
    origin_conn.execute(&get_drop_table_sql(TABLE_NAME), ())?;

    println!("Creating SQLite Table...");
    origin_conn.execute(&get_create_table_sql(TABLE_NAME), ())?;

    println!("Target url: {}", TARGET_URL);

    let conn = Arc::new(Mutex::new(origin_conn));

    let client = Arc::new(Client::new());

    // End: 148900
    // No data: 149000
    // TODO: Debug
    // let current_start = Arc::new(Mutex::new(146000));
    let current_start = Arc::new(Mutex::new(0));

    let mut handle_vec = Vec::with_capacity(num_cpus::get_physical());

    for n in 1..num_cpus::get_physical() {
        let current_start = Arc::clone(&current_start);

        let conn = Arc::clone(&conn);

        let client = Arc::clone(&client);

        handle_vec.push(tokio::spawn(async move {
            run_task(client, conn, current_start).await.unwrap();
        }));
    }

    for task in handle_vec {
        task.await.unwrap();
    }

    Ok(())
}

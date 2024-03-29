use anyhow::Result as AnyhowResult;
use num_cpus;
use reqwest::{self, Client};
use rusqlite::{Connection, Result as RSqlResult};
use scraper::{Html, Selector};
use std::sync::{Arc, Mutex};
use std::process;
use std::time::Duration;
use tokio;
use dotenv::dotenv;
use std::env;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

const TARGET_URL: &str = "http://rwr.runningwithrifles.com/rwr_stats/view_players.php";
const SELECTOR_MATCH: &str = "table > tbody > tr";
const PAGE_SIZE: i128 = 100;
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

async fn run_task(
    client: Arc<ClientWithMiddleware>,
    conn: Arc<Mutex<Connection>>,
    start: Arc<Mutex<i128>>,
    thread_n: usize,
    db: &str,
    delay: u64,
    timeout: u64
) -> AnyhowResult<()> {
    loop {
        // 先改值, 使得下一次数据正确
        let current_start = {
            let mut next_start = start.lock().unwrap();
            let start = *next_start;
            *next_start += PAGE_SIZE as i128;
            start
        };

        if delay > 0 {
            println!(">>>>> {}:Delay {} s >>>>>", thread_n, delay);
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }

        println!(
            ">>>>> {}:Sending Request... start:{} >>>>>",
            thread_n, current_start
        );
        let resp = client
            .get(TARGET_URL)
            .query(&[
                // invasion / pacific
                // ("db", "invasion"),
                ("db", db),
                ("sort", "rank_progression"),
                ("start", &current_start.to_string()),
            ])
            // .header("Host", "rwr.runningwithrifles.com")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/109.0")
            // .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8")
            // .header("Accept-Language", "zh,zh-CN;q=0.8,en-US;q=0.5,en;q=0.3")
            // .header("Accept-Encoding", "gzip, deflate")
            // .header("Connection", "keep-alive")
            // .header("Upgrade-Insecure-Requests", "1")
            .timeout(Duration::from_secs(timeout))
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
                ">>>>> {}:Start Parsing... start:{}, data(before):{} >>>>>",
                thread_n, current_start, data_size
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
                                    let times_str_iter = times_str_split_collect.iter().rev();

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
            ">>>>> {}:Parsing completed, start:{}, data(after):{} >>>>>",
            thread_n, current_start, data_size
        );

        if data_size < PAGE_SIZE {
            println!("===== {}:Parsing End===== ", thread_n);
            if data_size != -1 {
                println!(
                    "===== {}:No More data: end current + size: {} =====",
                    thread_n,
                    current_start - PAGE_SIZE + data_size
                );
            } else {
                println!(
                    "===== {}:No More data: end current: {} =====",
                    thread_n,
                    current_start - PAGE_SIZE
                );
            }
            return Ok(());
        }
    }
}

#[tokio::main]
async fn main() -> AnyhowResult<()> {
    dotenv().ok();

    // Parse ENV
    let start = env::var("START").unwrap_or("0".to_string()).parse::<i128>()?;
    let db = env::var("DB")?;
    let delay = env::var("DELAY").unwrap_or("1".to_string()).parse::<u64>()?;
    let timeout = env::var("TIMEOUT").unwrap_or("5".to_string()).parse::<u64>()?;
    let retry = env::var("RETRY").unwrap_or("3".to_string()).parse::<u32>()?;

    println!("ENV: DB={}, start={}, DELAY={}, TIMEOUT={}, RETRY={}", db, start, delay, timeout, retry);

    // Clear SQLite Table
    println!("Creating SQLite connection...");
    let origin_conn = Connection::open(DB_NAME)?;

    println!("Dropping SQLite Table...");
    origin_conn.execute(&get_drop_table_sql(TABLE_NAME), ())?;

    println!("Creating SQLite Table...");
    origin_conn.execute(&get_create_table_sql(TABLE_NAME), ())?;

    // Shared state
    println!("Target url: {}", TARGET_URL);

    let conn = Arc::new(Mutex::new(origin_conn));

    // reqwest client
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(retry);
    let client = ClientBuilder::new(Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    
    let client = Arc::new(client);

    let db_name = Arc::new(db);

    let delay = Arc::new(delay);

    let timeout = Arc::new(timeout);

    // Debug data param
    // End: 148900
    // No data: 149000
    // TODO: Debug
    // let current_start = Arc::new(Mutex::new(146000));
    // DLC
    // let current_start = Arc::new(Mutex::new(36000));
    let current_start = Arc::new(Mutex::new(start));

    let mut handle_vec = Vec::with_capacity(num_cpus::get_physical());


    for n in 0..num_cpus::get_physical() {
        let current_start = Arc::clone(&current_start);

        let conn = Arc::clone(&conn);

        let client = Arc::clone(&client);

        let db_name = Arc::clone(&db_name);

        let delay = Arc::clone(&delay);

        let timeout = Arc::clone(&timeout);

        handle_vec.push(tokio::spawn(async move {
            println!("##### Thread: {} start #####", n);
            match run_task(client, conn, current_start, n, &db_name, *delay, *timeout).await {
                Ok(()) => {
                    println!("##### Thread: {} end #####", n);
                    Ok(())
                },
                Err(e) => {
                    println!("##### Thread: {} err #####", n);
                    println!("{}", e.to_string());
                    process::exit(1);
                    Err(e)
                }
            }
        }));
    }

    for task in handle_vec {
        task.await.unwrap();
    }

    println!("All Threads({}) Completed", num_cpus::get_physical());

    Ok(())
}

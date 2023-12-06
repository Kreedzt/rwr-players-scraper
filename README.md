# RWR User Data Scraper

![license](https://badgen.net/github/license/Kreedzt/rwr-players-scraper)
![latest release](https://badgen.net/github/release/Kreedzt/rwr-players-scraper)

> This project crawls all data from http://rwr.runningwithrifles.com/rwr_stats/view_players.php official link, do not abuse it!

English | [简体中文](README_zhCN.md)

## Usage

You need to define the environment variables, and place the `.env` file in the same directory before running, it will merge the environment variables (refer to the `.env.example` file).

Environment variable parameter:
- DB: **required**, rwr's database, usually `pacific` or `invasion`.
- START: optional, starting data offset, default is 0.
- DELAY: optional, unit: seconds, wait time before each request, default is 1.
- TIMEOUT: optional, unit: seconds, wait time before each request, default is 5.
- RETRY: optional, the number of retries per failed request, default is 3.

**Empty data table** after each execution

After successful execution, the `rwr_players.db` file will be generated in the same directory in the format supported by SQLite3, which can be used by third-party database visualization tools to query the data, and the stored data table is named `rwr_players`.

## Development

This project is written in the Rust language and requires the [Rust](https://www.rust-lang.org/) development environment

Execute the following command in the project root directory to compile it:
``` sh
cargo run
```

> This project uses multi-threaded rounds to get all the data, the amount of data is large, the time required is long, please be patient when executing

After execution, the `rwr_players.db` file will be generated in the same directory in the format supported by SQLite3, which can be used to query the data with third-party database visualization tools.

## Build

This project is written in the Rust language and requires the [Rust](https://www.rust-lang.org/) development environment

Execute the following command in the project root directory to compile it:
```bash
cargo build --release
```

After compilation, a binary file (exe) is generated in `target/release` in the root directory.

## LICENSE

- [GPLv3](https://opensource.org/licenses/GPL-3.0)

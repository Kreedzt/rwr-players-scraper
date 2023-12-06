# RWR 用户数据爬取

![license](https://badgen.net/github/license/Kreedzt/rwr-players-scraper)
![latest release](https://badgen.net/github/release/Kreedzt/rwr-players-scraper)

> 该项目从 http://rwr.runningwithrifles.com/rwr_stats/view_players.php 官方链接爬取所有数据, 请勿滥用

[English](README.md) | 简体中文

## 使用

使用时需要定义环境变量, 运行前在同目录放置 `.env` 文件, 会合并环境变量(参考 `.env.example` 文件)

环境变量参数:
- DB: **必须**, rwr 网站数据库名称, 通常为 `pacific` 或 `invasion`
- START: 可选, 起始数据偏移, 默认为 0
- DELAY: 可选, 单位: 秒, 每次请求前等待时间, 默认为 1
- TIMEOUT: 可选, 单位: 秒, 每次请求超时时间, 默认为 5
- RETRY: 可选, 每次请求失败重试次数, 默认为 3

每次执行时会 **清空数据表**

执行成功后, 会在同目录下以 SQLite3 支持的格式生成 `rwr_players.db` 文件, 可用第三方数据库可视化工具查询数据, 存储的数据表名为 `rwr_players`

## 开发

该项目采用 Rust 语言编写，需要 [Rust](https://www.rust-lang.org/) 开发环境

在项目根目录下执行如下命令即可编译
``` sh
cargo run
```

> 该项目采用多线程轮训获取所有数据, 数据量较大, 所需时间较长, 执行时请耐心等待

执行后会在同目录下以 SQLite3 支持的格式生成 `rwr_players.db` 文件, 可用第三方数据库可视化工具查询数据

## 构建

该项目采用 Rust 语言编写，需要 [Rust](https://www.rust-lang.org/) 开发环境

编译需执行以下命令：
```bash
cargo build --release
```

编译后在根目录的 `target/release` 内生成二进制文件（exe）

## 协议

- [GPLv3](https://opensource.org/licenses/GPL-3.0)

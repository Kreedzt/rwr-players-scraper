# RWR 用户数据爬取

> 该项目从 http://rwr.runningwithrifles.com/rwr_stats/view_players.php 官方链接爬取所有数据, 请勿滥用

## 开发

该项目采用 Rust 语言编写，需要 [Rust](https://www.rust-lang.org/) 开发环境

该项目采用多线程轮训获取所有数据, 数据量较大, 所需时间较长, 可在 `src/main.rs` 中调整 `current_start` 起始数据偏移来方便测试

在项目根目录下执行如下命令即可编译
``` sh
cargo run
```

会在同目录下以 SQLite3 支持的格式生成 `rwr_players.db` 文件, 可用第三方数据库可视化工具查询数据

## 构建

该项目采用 Rust 语言编写，需要 [Rust](https://www.rust-lang.org/) 开发环境

编译需执行以下命令：
```bash
cargo build --release
```

编译后在根目录的 `target/release` 内生成二进制文件（exe）

## 协议

- [GPLv3](https://opensource.org/licenses/GPL-3.0)

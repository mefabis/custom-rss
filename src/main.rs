// SPDX-FileCopyrightText: 2025 Eduardo Martinez Martinez <eduardo@monte.blue>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use axum::{Router, routing::get};
use log::{error, info};
use std::net::SocketAddr;

mod isabel;
mod rss_utils;
mod verde;

const DEFAULT_ADDR: &str = "127.0.0.1:3101";

const HELP_MESSAGE: &str = r#"Custom RSS feed for web pages that don't have them.

Current feeds:
  /blog-isabel/feed       https://marmenormarmayor.es/El-blog-de-Isabel/index.html
  /verde/blog/feed        https://elclickverde.com/blog
  /verde/reportajes/feed  https://elclickverde.com/reportajes

Usage:
  $ custom-rss [-a <listening-addr>] [-h]

  -a <listening-addr> selects the IP and port that the server will listen to.
     Example 192.168.0.1:2612
     Default localhost:3101
  -h print help"#;

fn parse_args() -> Result<SocketAddr> {
    use lexopt::prelude::*;

    let mut addr: SocketAddr = DEFAULT_ADDR.parse()?;
    let mut parser = lexopt::Parser::from_env();

    while let Some(arg) = parser.next()? {
        match arg {
            Short('a') | Long("addr") => {
                addr = parser.value()?.parse()?;
            }
            Short('h') | Long("help") => {
                println!("{HELP_MESSAGE}");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected().into()),
        }
    }

    Ok(addr)
}

async fn run() -> Result<()> {
    let args = parse_args()?;
    info!("Listening on address: {args}");
    let app = Router::new()
        .route("/blog-isabel/feed", get(isabel::rss))
        .route("/verde/blog/feed", get(verde::blog_rss))
        .route("/verde/reportajes/feed", get(verde::reportajes_rss));
    let listener = tokio::net::TcpListener::bind(args).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    if let Err(e) = run().await {
        error!("Fatal error: {:#}", e);
        std::process::exit(1);
    }
}

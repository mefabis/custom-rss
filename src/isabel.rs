// SPDX-FileCopyrightText: 2025 Eduardo Martinez Martinez <eduardo@monte.blue>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Result, anyhow};
use axum::{http::StatusCode, response::IntoResponse};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono_tz::Europe::Madrid;
use log::error;
use rss::{ChannelBuilder, Guid, ItemBuilder};
use scraper::{Html, Selector};
use std::sync::LazyLock;

use crate::rss_utils;

const BLOG_URL_ARCHIVE: &str = "https://marmenormarmayor.es/El-blog-de-Isabel/archive.html";
const BLOG_URL_BLOG: &str = "https://marmenormarmayor.es/El-blog-de-Isabel/";
const BLOG_URL_PARSER_SECTION: &str = ".blogsection";
const BLOG_URL_PARSER_TITLE: &str = "h3.blogtitle a";
const BLOG_URL_PARSER_DATE: &str = ".blogdate";
const BLOG_URL_PARSER_CONTENT: &str = ".blogcontent";

static SELECTOR_SELECTION: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_SECTION).unwrap());
static SELECTOR_TITLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_TITLE).unwrap());
static SELECTOR_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_DATE).unwrap());
static SELECTOR_CONTENT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_CONTENT).unwrap());

fn parse_date(date: &str) -> Result<String> {
    let s: Vec<&str> = date.split_whitespace().collect();
    let day: u32 = s
        .get(1)
        .ok_or_else(|| anyhow!("Unable to extract day from input string"))?
        .parse()?;
    let year: i32 = s
        .get(5)
        .ok_or_else(|| anyhow!("Unable to extract year from input string"))?
        .parse()?;
    let month: u32 = match *(s
        .get(3)
        .ok_or_else(|| anyhow!("Unable to extract month from input string"))?)
    {
        "enero" => Ok(1),
        "febrero" => Ok(2),
        "marzo" => Ok(3),
        "abril" => Ok(4),
        "mayo" => Ok(5),
        "junio" => Ok(6),
        "julio" => Ok(7),
        "agosto" => Ok(8),
        "septiembre" => Ok(9),
        "octubre" => Ok(10),
        "noviembre" => Ok(11),
        "diciembre" => Ok(12),
        invalid => Err(anyhow!("Not a valid month: {invalid}")),
    }?;

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        anyhow!("Unable to form date with values year = {year}, month = {month} and day = {day}")
    })?;
    let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
    match NaiveDateTime::new(date, time).and_local_timezone(Madrid) {
        chrono::LocalResult::Single(dt) => Ok(dt.to_rfc2822()),
        _ => Err(anyhow!("Invalid timezone")),
    }
}

async fn parse_content() -> Result<impl IntoResponse> {
    let content = reqwest::get(BLOG_URL_ARCHIVE).await?.text().await?;
    let document = Html::parse_document(&content);

    let mut rss_channel = ChannelBuilder::default()
        .title("El blog de Isabel")
        .link(BLOG_URL_BLOG)
        .description("Últimas entradas del blog de Isabel")
        .build();

    for element in document.select(&SELECTOR_SELECTION) {
        let title = element
            .select(&SELECTOR_TITLE)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .ok_or_else(|| anyhow!("Unable to parse title"))?;

        let date = element
            .select(&SELECTOR_DATE)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .ok_or_else(|| anyhow!("Unable to parse date"))?;
        let date = parse_date(date.as_str())?;

        let content = element
            .select(&SELECTOR_CONTENT)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .ok_or_else(|| anyhow!("Unable to parse content"))?;

        let link = element
            .select(&SELECTOR_TITLE)
            .next()
            .and_then(|e| e.value().attr("href"))
            .map(|s| format!("{}{}", BLOG_URL_BLOG, s))
            .ok_or_else(|| anyhow!("Unable to parse link"))?;

        let item = ItemBuilder::default()
            .title(title)
            .link(link.clone())
            .description(content)
            .guid(Some(Guid {
                value: link,
                permalink: true,
            }))
            .pub_date(date)
            .build();

        rss_channel.items.push(item);
    }

    rss_utils::make_rss(rss_channel)
}

pub async fn rss() -> impl IntoResponse {
    parse_content()
        .await
        .map(|resp| resp.into_response())
        .unwrap_or_else(|e| {
            error!("Error parsing the content of the HTML: {e}");
            StatusCode::NO_CONTENT.into_response()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_lock_no_panic() {
        let dull_document = Html::new_document();
        let _ = dull_document.select(&SELECTOR_SELECTION);
        let _ = dull_document.select(&SELECTOR_TITLE);
        let _ = dull_document.select(&SELECTOR_DATE);
        let _ = dull_document.select(&SELECTOR_CONTENT);
    }
}

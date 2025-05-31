// SPDX-FileCopyrightText: 2025 Eduardo Martinez Martinez <eduardo@monte.blue>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Result, anyhow};
use axum::{http::StatusCode, response::IntoResponse};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono_tz::Europe::Madrid;
use log::error;
use rss::{ChannelBuilder, Guid, ItemBuilder};
use scraper::{Html, Selector, selectable::Selectable};
use std::sync::LazyLock;

use crate::rss_utils;

const MAIN_URL: &str = "https://elclickverde.com";

// blog
const BLOG_URL: &str = "https://elclickverde.com/blog";
const BLOG_URL_PARSER_ENTRY: &str = ".views-row";
const BLOG_URL_PARSER_HEADER: &str = ".group-header";
const BLOG_URL_PARSER_EVEN_HEADER: &str = ".field__item.even h2 a";
const BLOG_URL_PARSER_RIGHT: &str = ".group-right";
const BLOG_URL_PARSER_EVEN_DESCRIPTION: &str = "p:not(.rteright)";
const BLOG_URL_PARSER_EVEN_DATE: &str = "p.rteright span";

static BLOG_SELECTOR_ENTRY: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_ENTRY).unwrap());
static BLOG_SELECTOR_HEADER: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_HEADER).unwrap());
static BLOG_SELECTOR_EVEN_HEADER: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_EVEN_HEADER).unwrap());
static BLOG_SELECTOR_RIGHT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_RIGHT).unwrap());
static BLOG_SELECTOR_EVEN_DESCRIPTION: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_EVEN_DESCRIPTION).unwrap());
static BLOG_SELECTOR_EVEN_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(BLOG_URL_PARSER_EVEN_DATE).unwrap());

// reportajes
const REPORTAJES_URL: &str = "https://elclickverde.com/reportajes";
const REPORTAJES_URL_PARSER_ENTRY: &str = ".views-row";
const REPORTAJES_URL_PARSER_HEADER: &str = r#"div.field__item.even[property="dc:title"] h2 a"#;
const REPORTAJES_URL_PARSER_DESCRIPTION: &str =
    r#"div.field__item.even[property="content:encoded"]"#;
const REPORTAJES_URL_PARSER_DATE: &str = "div.field.field--name-post-date";
const REPORTAJES_URL_PARSER_INNER_DATE: &str = "div.field__item.even";

static REPORTAJES_SELECTOR_ENTRY: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(REPORTAJES_URL_PARSER_ENTRY).unwrap());
static REPORTAJES_SELECTOR_HEADER: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(REPORTAJES_URL_PARSER_HEADER).unwrap());
static REPORTAJES_SELECTOR_DESCRIPTION: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(REPORTAJES_URL_PARSER_DESCRIPTION).unwrap());
static REPORTAJES_SELECTOR_P: LazyLock<Selector> = LazyLock::new(|| Selector::parse("p").unwrap());
static REPORTAJES_SELECTOR_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(REPORTAJES_URL_PARSER_DATE).unwrap());
static REPORTAJES_SELECTOR_INNER_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(REPORTAJES_URL_PARSER_INNER_DATE).unwrap());

async fn parse_blog() -> Result<impl IntoResponse> {
    let content = reqwest::get(BLOG_URL).await?.text().await?;
    let document = Html::parse_document(&content);

    let mut rss_channel = ChannelBuilder::default()
        .title("Blog | elclickverde")
        .link(BLOG_URL)
        .description("Últimas entradas del blog de elclickverde")
        .build();

    for element in document.select(&BLOG_SELECTOR_ENTRY) {
        let (title, url) = {
            let header = element
                .select(&BLOG_SELECTOR_HEADER)
                .next()
                .and_then(|h| h.select(&BLOG_SELECTOR_EVEN_HEADER).next())
                .ok_or_else(|| anyhow!("Unable to parse header"))?;

            let url = header
                .value()
                .attr("href")
                .map(|s| format!("{MAIN_URL}{s}"))
                .ok_or_else(|| anyhow!("Unable to extract URL"))?;

            let title = header.text().collect::<String>().trim().to_string();

            (title, url)
        };

        let (description, date) = {
            let right = element
                .select(&BLOG_SELECTOR_RIGHT)
                .next()
                .ok_or_else(|| anyhow!("Unable to parse right"))?;

            let description = right
                .select(&BLOG_SELECTOR_EVEN_DESCRIPTION)
                .next()
                .map(|p| p.text().collect::<String>().trim().to_string())
                .ok_or_else(|| anyhow!("Unable to parse even description"))?;

            let date = {
                let text = right
                    .select(&BLOG_SELECTOR_EVEN_DATE)
                    .next()
                    .map(|span| span.text().collect::<String>().trim().to_string())
                    .ok_or_else(|| anyhow!("Unable to parse even date"))?;

                let fecha = text.parse::<NaiveDate>()?;
                let time = NaiveTime::from_hms_opt(12, 0, 0).unwrap_or_default();
                match NaiveDateTime::new(fecha, time).and_local_timezone(Madrid) {
                    chrono::LocalResult::Single(dt) => Ok(dt.to_rfc2822()),
                    _ => Err(anyhow!("Invalid timezone")),
                }
            }?;

            (description, date)
        };

        let item = ItemBuilder::default()
            .title(title)
            .link(url.clone())
            .description(description)
            .guid(Some(Guid {
                value: url,
                permalink: true,
            }))
            .pub_date(date)
            .build();

        rss_channel.items.push(item);
    }

    rss_utils::make_rss(rss_channel)
}

fn reportajes_date(input: &str) -> Result<String> {
    let s: Vec<&str> = input.split_whitespace().collect();
    let day: u32 = s
        .first()
        .ok_or_else(|| anyhow!("Unable to extract day from input string"))?
        .parse()?;
    let year: i32 = s
        .get(2)
        .ok_or_else(|| anyhow!("Unable to extract year from input string"))?
        .parse()?;
    let month: u32 = match *(s
        .get(1)
        .ok_or_else(|| anyhow!("Unable to extract month from input string"))?)
    {
        "Ene." => Ok(1),
        "Feb." => Ok(2),
        "Mar." => Ok(3),
        "Abr." => Ok(4),
        "Mayo." => Ok(5),
        "Jun." => Ok(6),
        "Jul." => Ok(7),
        "Ago." => Ok(8),
        "Sep." => Ok(9),
        "Oct." => Ok(10),
        "Nov." => Ok(11),
        "Dic." => Ok(12),
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

async fn parse_reportajes() -> Result<impl IntoResponse> {
    let content = reqwest::get(REPORTAJES_URL).await?.text().await?;
    let document = Html::parse_document(&content);

    let mut rss_channel = ChannelBuilder::default()
        .title("Reportajes | elclickverde")
        .link(REPORTAJES_URL)
        .description("Últimos reportajes de elclickverde")
        .build();

    for element in document.select(&REPORTAJES_SELECTOR_ENTRY) {
        let (title, url) = {
            let header = element
                .select(&REPORTAJES_SELECTOR_HEADER)
                .next()
                .ok_or_else(|| anyhow!("Unable to parse header"))?;

            let url = header
                .value()
                .attr("href")
                .map(|s| format!("{MAIN_URL}{s}"))
                .ok_or_else(|| anyhow!("Unable to extract URL"))?;

            let title = header.text().collect::<String>().trim().to_string();

            (title, url)
        };

        let description = {
            let desc = element
                .select(&REPORTAJES_SELECTOR_DESCRIPTION)
                .next()
                .ok_or_else(|| anyhow!("Unable to parse description"))?;
            let mut p_tags = desc.select(&REPORTAJES_SELECTOR_P);
            p_tags
                .nth(1)
                .map(|tag| tag.text().collect::<String>().trim().to_string())
                .ok_or_else(|| anyhow!("Unable to extract description"))?
        };

        let date = {
            let div_date = element
                .select(&REPORTAJES_SELECTOR_DATE)
                .next()
                .and_then(|h| h.select(&REPORTAJES_SELECTOR_INNER_DATE).next())
                .ok_or_else(|| anyhow!("Unable to parse inner date"))?;

            let date_raw = div_date.text().collect::<String>().trim().to_string();

            reportajes_date(&date_raw)?
        };

        let item = ItemBuilder::default()
            .title(title)
            .link(url.clone())
            .description(description)
            .guid(Some(Guid {
                value: url,
                permalink: true,
            }))
            .pub_date(date)
            .build();

        rss_channel.items.push(item);
    }

    rss_utils::make_rss(rss_channel)
}

pub async fn blog_rss() -> impl IntoResponse {
    parse_blog()
        .await
        .map(|resp| resp.into_response())
        .unwrap_or_else(|e| {
            error!("Error parsing the content of the HTML: {e}");
            StatusCode::NO_CONTENT.into_response()
        })
}

pub async fn reportajes_rss() -> impl IntoResponse {
    parse_reportajes()
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

        // Blog
        let _ = dull_document.select(&BLOG_SELECTOR_ENTRY);
        let _ = dull_document.select(&BLOG_SELECTOR_HEADER);
        let _ = dull_document.select(&BLOG_SELECTOR_EVEN_HEADER);
        let _ = dull_document.select(&BLOG_SELECTOR_RIGHT);
        let _ = dull_document.select(&BLOG_SELECTOR_EVEN_DESCRIPTION);

        // Reportajes
        let _ = dull_document.select(&REPORTAJES_SELECTOR_ENTRY);
        let _ = dull_document.select(&REPORTAJES_SELECTOR_HEADER);
        let _ = dull_document.select(&REPORTAJES_SELECTOR_DESCRIPTION);
        let _ = dull_document.select(&REPORTAJES_SELECTOR_P);
        let _ = dull_document.select(&REPORTAJES_SELECTOR_INNER_DATE);
    }
}

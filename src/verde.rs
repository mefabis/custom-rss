// SPDX-FileCopyrightText: 2025 Eduardo Martinez Martinez <eduardo@monte.blue>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result, anyhow};
use axum::{http::StatusCode, response::IntoResponse};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use chrono_tz::Europe::Madrid;
use log::error;
use rss::{ChannelBuilder, Guid, ItemBuilder};
use scraper::{Html, Selector, selectable::Selectable};

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

// reportajes
const REPORTAJES_URL: &str = "https://elclickverde.com/reportajes";
const REPORTAJES_URL_PARSER_ENTRY: &str = ".views-row";
const REPORTAJES_URL_PARSER_HEADER: &str = r#"div.field__item.even[property="dc:title"] h2 a"#;
const REPORTAJES_URL_PARSER_DESCRIPTION: &str =
    r#"div.field__item.even[property="content:encoded"]"#;
const REPORTAJES_URL_PARSER_DATE: &str = "div.field.field--name-post-date";
const REPORTAJES_URL_PARSER_INNER_DATE: &str = "div.field__item.even";

async fn parse_blog() -> Result<impl IntoResponse> {
    let content = reqwest::get(BLOG_URL).await?.text().await?;
    let document = Html::parse_document(&content);
    let selector_entry = Selector::parse(BLOG_URL_PARSER_ENTRY)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse entry selector")?;
    let selector_header = Selector::parse(BLOG_URL_PARSER_HEADER)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse header selector")?;
    let selector_even_header = Selector::parse(BLOG_URL_PARSER_EVEN_HEADER)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse even header selector")?;
    let selector_right = Selector::parse(BLOG_URL_PARSER_RIGHT)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse right selector")?;
    let selector_even_description = Selector::parse(BLOG_URL_PARSER_EVEN_DESCRIPTION)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse even description selector")?;
    let selector_even_date = Selector::parse(BLOG_URL_PARSER_EVEN_DATE)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse date selector")?;

    let mut rss_channel = ChannelBuilder::default()
        .title("Blog | elclickverde")
        .link(BLOG_URL)
        .description("Últimas entradas del blog de elclickverde")
        .build();

    for element in document.select(&selector_entry) {
        let (title, url) = {
            let header = element
                .select(&selector_header)
                .next()
                .and_then(|h| h.select(&selector_even_header).next())
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
                .select(&selector_right)
                .next()
                .ok_or_else(|| anyhow!("Unable to parse right"))?;

            let description = right
                .select(&selector_even_description)
                .next()
                .map(|p| p.text().collect::<String>().trim().to_string())
                .ok_or_else(|| anyhow!("Unable to parse even description"))?;

            let date = {
                let text = right
                    .select(&selector_even_date)
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

    let selector_entry = Selector::parse(REPORTAJES_URL_PARSER_ENTRY)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse entry selector")?;
    let selector_header = Selector::parse(REPORTAJES_URL_PARSER_HEADER)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse header selector")?;
    let selector_description = Selector::parse(REPORTAJES_URL_PARSER_DESCRIPTION)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse description selector")?;
    let selector_p = Selector::parse("p")
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse p selector")?;
    let selector_date = Selector::parse(REPORTAJES_URL_PARSER_DATE)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse date selector")?;
    let selector_inner_date = Selector::parse(REPORTAJES_URL_PARSER_INNER_DATE)
        .map_err(|e| anyhow!(e.to_string()))
        .context("Failed to parse inner date selector")?;

    for element in document.select(&selector_entry) {
        let (title, url) = {
            let header = element
                .select(&selector_header)
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
                .select(&selector_description)
                .next()
                .ok_or_else(|| anyhow!("Unable to parse description"))?;
            let mut p_tags = desc.select(&selector_p);
            p_tags
                .nth(1)
                .map(|tag| tag.text().collect::<String>().trim().to_string())
                .ok_or_else(|| anyhow!("Unable to extract description"))?
        };

        let date = {
            let div_date = element
                .select(&selector_date)
                .next()
                .and_then(|h| h.select(&selector_inner_date).next())
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

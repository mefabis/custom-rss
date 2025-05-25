// SPDX-FileCopyrightText: 2025 Eduardo Martinez Martinez <eduardo@monte.blue>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use axum::{
    http::{
        StatusCode,
        header::{CONTENT_TYPE, HeaderMap},
    },
    response::IntoResponse,
};
use rss::Channel;

const XML_TYPE: &str = "application/xml";

pub fn make_rss(ch: Channel) -> Result<impl IntoResponse> {
    let mut headers = HeaderMap::new();
    let xml = XML_TYPE.parse()?;
    headers.insert(CONTENT_TYPE, xml);
    Ok((StatusCode::OK, headers, ch.to_string()))
}

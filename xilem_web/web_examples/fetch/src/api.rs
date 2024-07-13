// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cat {
    pub url: String,
}

#[derive(Error, Clone, Debug)]
pub enum CatError {
    #[error("Please request more than zero cats.")]
    NonZeroCats,
}

pub type CatCount = usize;

pub async fn fetch_cats(count: CatCount) -> anyhow::Result<Vec<Cat>> {
    log::debug!("Fetch {count} cats");
    if count < 1 {
        return Err(CatError::NonZeroCats.into());
    }
    let url = format!("https://api.thecatapi.com/v1/images/search?limit={count}",);
    Ok(Request::get(&url)
        .send()
        .await?
        .json::<Vec<Cat>>()
        .await?
        .into_iter()
        .take(count)
        .collect())
}

use futures::future::BoxFuture;
use itertools::Itertools;
use lapin::message::Delivery;
use lockable::{AsyncLimit, LockableHashMap};
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;
use uuid::Uuid;

use super::{entity::summary, trie::Tree, Error};
use crate::constant::*;

/// Heartbeat is a struct that contains the pathline and time of a heartbeat
///
/// It's part of API spec
#[derive(Deserialize, Serialize)]
pub struct Heartbeat {
    #[serde(rename = "path")]
    pathline: String,
    entity: Option<String>,
    category: Option<String>,
    browser: Option<String>,
    domain: Option<String>,
    user_agent: Option<String>,
    time: Time,
    created_at: Time,
}

/// Heartbeat is a struct that contains the pathline and time of a heartbeat
///
/// It's part of API spec
#[derive(Deserialize, Serialize)]
pub struct Heartbeats {
    pub trace_id: u64,
    pub user_id: Uuid,
    list: Vec<Heartbeat>,
}

impl TryFrom<Delivery> for Heartbeats {
    type Error = Error;

    fn try_from(value: Delivery) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value.data).map_err(Error::RequestParseError)
    }
}

/// Beatbuffer realises logic of batching heartbeats and uploading them to the database
#[derive(Default)]
pub struct Beatbuffer {
    start: Time,
    end: Time,
    beats: Vec<Heartbeat>,
}

impl From<Heartbeats> for Beatbuffer {
    fn from(value: Heartbeats) -> Self {
        let now = chrono::offset::Local::now().fixed_offset();
        let start = value.list.iter().map(|x| x.time).min().unwrap_or(now);
        let end = value.list.iter().map(|x| x.time).min().unwrap_or(now);
        Self {
            start,
            end,
            beats: value.list,
        }
    }
}

impl Beatbuffer {
    pub fn add(&mut self, beats: Heartbeats) {
        self.beats.extend(beats.list);
    }
    pub fn is_full(&self) -> bool {
        match self.beats.is_empty() {
            true => false,
            false => {
                (self.beats.len() >= BUFFER_MAX_LENGTH)
                    || (self.end - self.start >= BUFFER_MAX_TIME)
            }
        }
    }
    pub(super) fn into_payloads(self) -> impl Iterator<Item = (summary::ActiveModel, Tree)> {
        let mut result = Vec::new();
        for (_, beats) in self
            .beats
            .into_iter()
            .group_by(|x| (x.browser.clone(), x.category.clone(), x.entity.clone()))
            .into_iter()
        {
            let tree = Tree::from_iter(beats.map(|x| x.pathline));
            let summary = summary::ActiveModel {
                from_time: ActiveValue::Set(self.start.naive_local().time()),
                to_time: ActiveValue::Set(self.end.naive_local().time()),
                ..Default::default()
            };
            result.push((summary, tree));
        }
        result.into_iter()
    }
}
/// BeatBuffers is a struct that enable batching heartbeats and uploading them to the database
#[derive(Default)]
pub struct BeatBuffers(LockableHashMap<Uuid, Beatbuffer>);

impl BeatBuffers {
    pub fn new() -> Self {
        Self(LockableHashMap::new())
    }
    /// partially lock the hashmap, insert the beatbuffer if the user_id is not present
    ///
    /// return the beatbuffer if it should be flushed
    pub async fn add(&self, beats: Heartbeats) -> Option<Beatbuffer> {
        let mut entry = self
            .0
            .async_lock(beats.user_id, AsyncLimit::no_limit())
            .await
            .unwrap();
        match entry.value_mut() {
            Some(x) => x.add(beats),
            None => {
                entry.insert(Beatbuffer::from(beats));
            }
        };
        match entry.value().unwrap().is_full() {
            true => Some(entry.remove().unwrap()),
            false => None,
        }
    }
    pub async fn get_full(&self, uuid: Uuid) -> Option<Beatbuffer> {
        let mut entry = self
            .0
            .async_lock(uuid, AsyncLimit::no_limit())
            .await
            .unwrap();
        match entry.value_mut() {
            Some(beat) if beat.is_full() => entry.remove(),
            _ => None,
        }
    }
    /// force flush all the beatbuffer in the hashmap
    pub async fn flush(&self, f: impl Fn(Beatbuffer) -> BoxFuture<'static, ()>) {
        let mut join_set = tokio::task::JoinSet::<()>::new();

        let mut iter = self.0.lock_all_entries().await;
        while let Some(mut entry) = iter.next().await {
            let beats = entry.remove().unwrap();
            join_set.spawn(f(beats));
        }

        while join_set.join_next().await.is_some() {}
    }
}

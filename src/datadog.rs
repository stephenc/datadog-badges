extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate serde_urlencoded;

use std::cmp::{max, Ordering};
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use regex::RegexSet;
use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub async fn get_monitor_details(
    client: &Client,
    api_key: &str,
    app_key: &str,
    id: &str,
    with_groups: bool,
) -> Result<Response, Error> {
    let url = if with_groups {
        format!("https://api.datadoghq.com/api/v1/monitor/{}", id)
    } else {
        format!(
            "https://api.datadoghq.com/api/v1/monitor/{}?group_states=all",
            id
        )
    };
    client
        .get(&url)
        .header("DD-API-KEY", api_key.to_owned())
        .header("DD-APPLICATION-KEY", app_key.to_owned())
        .send()
        .await
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Copy)]
pub enum MonitorStatus {
    #[serde(rename = "Ignored")]
    Ignored,
    #[serde(rename = "Skipped")]
    Skipped,
    #[serde(rename = "OK", alias = "Ok")]
    Ok,
    #[serde(rename = "No Data")]
    NoData,
    #[serde(rename = "Warn")]
    Warn,
    #[serde(rename = "Alert")]
    Alert,
    #[serde(rename = "Unknown")]
    Unknown,
}

impl Default for MonitorStatus {
    fn default() -> Self {
        MonitorStatus::Ok
    }
}

impl Ord for MonitorStatus {
    fn cmp(&self, other: &Self) -> Ordering {
        use MonitorStatus::*;
        match self {
            Ignored => match other {
                Ignored => Ordering::Equal,
                Skipped | Ok | NoData | Warn | Alert | Unknown => Ordering::Less,
            },
            Skipped => match other {
                Ignored => Ordering::Greater,
                Skipped => Ordering::Equal,
                Ok | NoData | Warn | Alert | Unknown => Ordering::Less,
            },
            Ok => match other {
                Skipped | Ignored => Ordering::Greater,
                Ok => Ordering::Equal,
                NoData | Warn | Alert | Unknown => Ordering::Less,
            },
            NoData => match other {
                Skipped | Ignored | Ok => Ordering::Greater,
                NoData => Ordering::Equal,
                Warn | Alert | Unknown => Ordering::Less,
            },
            Warn => match other {
                Skipped | Ignored | Ok | NoData => Ordering::Greater,
                Warn => Ordering::Equal,
                Alert | Unknown => Ordering::Less,
            },
            Alert => match other {
                Skipped | Ignored | Ok | NoData | Warn => Ordering::Greater,
                Alert => Ordering::Equal,
                Unknown => Ordering::Less,
            },
            Unknown => match other {
                Skipped | Ignored | Ok | NoData | Warn | Alert => Ordering::Greater,
                Unknown => Ordering::Equal,
            },
        }
    }
}

impl PartialOrd for MonitorStatus {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MonitorState {
    #[serde(default)]
    pub overall_state: MonitorStatus,
    #[serde(default, with = "rfc3339_date_format")]
    pub overall_state_modified: Option<DateTime<Utc>>,
    #[serde(default, with = "rfc3339_date_format")]
    pub modified: Option<DateTime<Utc>>,
    #[serde(default)]
    pub options: MonitorOptions,
    #[serde(default)]
    pub state: Option<MonitorStateDetail>,
}

impl Default for MonitorState {
    fn default() -> Self {
        MonitorState {
            overall_state: MonitorStatus::default(),
            overall_state_modified: None,
            modified: None,
            options: MonitorOptions::default(),
            state: None,
        }
    }
}

fn filter_tag_as_regex(tag: &str) -> String {
    match tag.find(':') {
        None => format!(
            "^{}",
            regex::escape(tag).replace("\\*", ".*").replace("\\?", ".")
        ),
        Some(index) => {
            let name = regex::escape(&tag[..index]);
            let value = regex::escape(&tag[index + 1..])
                .replace("\\*", ".*")
                .replace("\\?", ".");
            format!("^{}:{}", name, value)
        }
    }
}

fn filter_as_regexs(filter: &str) -> Option<Vec<String>> {
    if filter.is_empty() {
        None
    } else {
        Some(filter.split_whitespace().map(filter_tag_as_regex).collect())
    }
}

impl MonitorState {
    pub fn status(&self, filter: Option<&str>) -> (MonitorStatus, Option<DateTime<Utc>>) {
        let filter = match filter {
            None => None,
            Some(filter) => match filter_as_regexs(filter) {
                None => None,
                Some(set) => RegexSet::new(&set).map_or(None, Some),
            },
        };
        match &self.state {
            None => (self.overall_state, self.overall_state_modified),
            Some(state) => match &state.groups {
                Some(groups) => {
                    let mut filtered: Vec<(MonitorStatus, Option<DateTime<Utc>>)> = groups
                        .iter()
                        .filter(|(k, _)| match &filter {
                            Some(s) => k.split(",").any(|k| s.is_match(k)),
                            None => true,
                        })
                        .map(|(_, v)| match v.status {
                            MonitorStatus::Ok => (
                                MonitorStatus::Ok,
                                match (v.last_resolved_ts, self.overall_state_modified) {
                                    (None, None) => None,
                                    (None, Some(t)) | (Some(t), None) => Some(t),
                                    (Some(t1), Some(t2)) => Some(max(t1, t2)),
                                },
                            ),
                            MonitorStatus::NoData => (
                                MonitorStatus::NoData,
                                match (
                                    v.last_nodata_ts,
                                    v.last_triggered_ts,
                                    self.overall_state_modified,
                                ) {
                                    (None, None, None) => None,
                                    (None, None, Some(t))
                                    | (None, Some(t), None)
                                    | (Some(t), None, None) => Some(t),
                                    (Some(t1), Some(t2), None)
                                    | (Some(t1), None, Some(t2))
                                    | (None, Some(t1), Some(t2)) => Some(max(t1, t2)),
                                    (Some(t1), Some(t2), Some(t3)) => Some(max(t1, max(t2, t3))),
                                },
                            ),
                            _ => (
                                v.status,
                                match (v.last_triggered_ts, self.overall_state_modified) {
                                    (None, None) => None,
                                    (None, Some(t)) | (Some(t), None) => Some(t),
                                    (Some(t1), Some(t2)) => Some(max(t1, t2)),
                                },
                            ),
                        })
                        .collect();
                    if filtered.is_empty() {
                        (MonitorStatus::NoData, None)
                    } else {
                        filtered.sort_by(|(a, _), (b, _)| b.cmp(&a));
                        filtered
                            .first()
                            .unwrap_or_else(|| &(MonitorStatus::NoData, None))
                            .to_owned()
                    }
                }
                None => (self.overall_state, self.overall_state_modified),
            },
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct MonitorOptions {
    pub silenced: BTreeMap<String, Value>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct MonitorStateDetail {
    pub groups: Option<BTreeMap<String, MonitorGroupState>>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MonitorGroupState {
    pub status: MonitorStatus,
    #[serde(with = "posix_date_format")]
    pub last_triggered_ts: Option<DateTime<Utc>>,
    #[serde(with = "posix_date_format")]
    pub last_nodata_ts: Option<DateTime<Utc>>,
    #[serde(with = "posix_date_format")]
    pub last_notified_ts: Option<DateTime<Utc>>,
    #[serde(with = "posix_date_format")]
    pub last_resolved_ts: Option<DateTime<Utc>>,
}

mod rfc3339_date_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => {
                let s = date.to_rfc3339();
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer) {
            Ok(s) => DateTime::parse_from_rfc3339(&s)
                .map(|l| Some(l.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom),
            Err(_) => Ok(None),
        }
    }
}

mod posix_date_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => serializer.serialize_i64(date.timestamp()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match i64::deserialize(deserializer) {
            Ok(t) => Ok(Some(Utc.timestamp(t, 0))),
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use crate::datadog::{filter_tag_as_regex, MonitorState, MonitorStatus};

    #[test]
    fn test_tag_to_regex() {
        assert_eq!(filter_tag_as_regex("env"), r"^env");
        assert_eq!(filter_tag_as_regex("env.foo*"), r"^env\.foo.*");
        assert_eq!(filter_tag_as_regex("env:*"), r"^env:.*");
    }

    #[test]
    fn test_deserialize() {
        // I have a feeling that the sample response from
        // https://docs.datadoghq.com/api/?lang=bash#get-a-monitor-s-details
        // is no longer likely, but we should test against it anyway
        // such a shame that they do not provide a schema
        let v: MonitorState = serde_json::from_str(include_str!("test_data/sample.json")).unwrap();
        assert_eq!(v.status(None), (MonitorStatus::Alert, Some( DateTime::parse_from_rfc3339("2016-12-16T17:26:00Z").unwrap().with_timezone(&Utc))));
        assert_eq!(v.status(Some("host:host0")), (MonitorStatus::Alert, Some( DateTime::parse_from_rfc3339("2016-12-16T17:26:00Z").unwrap().with_timezone(&Utc))));
    }
}

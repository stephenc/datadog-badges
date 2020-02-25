extern crate reqwest;
extern crate serde;

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use reqwest::{Client, Error, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub async fn get_monitor_details(client: &Client, api_key: &str, app_key: &str, id: &str) -> Result<Response, Error> {
    let url = format!("https://api.datadoghq.com/api/v1/monitor/{}", id);
    client.get(&url)
        .header("DD-API-KEY", api_key.to_owned())
        .header("DD-APPLICATION-KEY", app_key.to_owned())
        .send()
        .await
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct MonitorState {
    pub overall_state: String,
    #[serde(with = "rfc3339_date_format")]
    pub overall_state_modified: Option<DateTime<Utc>>,
    pub options: MonitorOptions,
}

impl Default for MonitorState {
    fn default() -> Self {
        MonitorState {
            overall_state: "".to_owned(),
            overall_state_modified: None,
            options: MonitorOptions::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct MonitorOptions {
    pub silenced: BTreeMap<String, Value>,
}

mod rfc3339_date_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        date: &Option<DateTime<Utc>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        match date {
            Some(date) => {
                let s = date.to_rfc3339();
                serializer.serialize_str(&s)
            }
            None => serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<DateTime<Utc>>, D::Error>
        where
            D: Deserializer<'de>,
    {
        match String::deserialize(deserializer) {
            Ok(s) => DateTime::parse_from_rfc3339(&s).map(|l| Some(l.with_timezone(&Utc))).map_err(serde::de::Error::custom),
            Err(_) => Ok(None)
        }
    }
}

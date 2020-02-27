extern crate cached;
extern crate datadog_badges;
extern crate env_logger;
#[macro_use]
extern crate log;

use std::collections::BTreeMap;
use std::env;
use std::net::ToSocketAddrs;
use std::process::exit;
use std::sync::Mutex;

use cached::once_cell::sync::Lazy;
use cached::{Cached, TimedCache};
use chrono::Utc;
use env_logger::Env;
use getopts::Options;
use regex::Regex;
use warp::reject::not_found;
use warp::{http::Response, Filter, Rejection};

use datadog_badges::badge::{
    Badge, BadgeOptions, COLOR_DANGER, COLOR_OTHER, COLOR_SUCCESS, COLOR_WARNING,
};
use datadog_badges::datadog::{get_monitor_details, MonitorState, MonitorStatus};

async fn get_monitor_badge(
    status_codes: bool,
    account: String,
    id: String,
    query: BTreeMap<String, String>,
) -> Result<Response<String>, Rejection> {
    static MAX_AGE_SECONDS: Lazy<u64> = Lazy::new(|| match env::var("CACHE_TTL_SECONDS") {
        Ok(value) => value.parse::<u64>().unwrap_or(15),
        Err(_) => 15,
    });
    type CacheKey = (String, String, BTreeMap<String, String>);
    type CacheValue = (BadgeOptions, u16);
    type BadgeCache = TimedCache<CacheKey, CacheValue>;
    static BADGE_CACHE: Lazy<Mutex<BadgeCache>> =
        Lazy::new(|| Mutex::new(TimedCache::with_lifespan(*MAX_AGE_SECONDS)));

    let mut query = query.clone();
    query.remove("badge-poll");
    let query = query;

    let key = (account.clone(), id.clone(), query.clone());
    let max_age: u64 = *MAX_AGE_SECONDS;
    {
        let mut cache = BADGE_CACHE.lock().unwrap();
        if let Some((options, status_code)) = cache.cache_get(&key) {
            return Response::builder()
                .status(*status_code)
                .header("Content-Type", "image/svg+xml")
                .header("Cache-Control", format!("public,max-age={}", max_age))
                .body(Badge::new(options.clone()).to_svg())
                .map_err(|_| not_found());
        }
    }
    let client = reqwest::Client::new();
    let env_root = account.to_string().to_uppercase();
    let env_root = Regex::new(r"[^A-Z0-9_]")
        .unwrap()
        .replace_all(&env_root, "_");
    let app_key = env::var(format!("{}_DATADOG_APP_KEY", env_root));
    let api_key = env::var(format!("{}_DATADOG_API_KEY", env_root));
    let value = if let (Ok(api_key), Ok(app_key)) = (api_key, app_key) {
        let details =
            get_monitor_details(&client, &api_key, &app_key, &id, query.get("g").is_some()).await;
        match details {
            Err(_) => (
                BadgeOptions {
                    status: "HTTP/500 Internal Server Error".to_owned(),
                    color: COLOR_WARNING.to_owned(),
                    ..BadgeOptions::default()
                },
                if status_codes { 500 } else { 200 },
            ),
            Ok(response) => {
                if response.status().is_success() {
                    let value: MonitorState = response.json().await.map_err(|_| not_found())?;
                    let (status, since) = value.status(query.get("q").map(String::as_ref));
                    (
                        BadgeOptions {
                            duration: match since {
                                Some(v) => Some(Utc::now().signed_duration_since(v)),
                                None => None,
                            },
                            color: match &status {
                                MonitorStatus::Ok | MonitorStatus::Skipped => {
                                    COLOR_SUCCESS.to_owned()
                                }
                                MonitorStatus::Alert | MonitorStatus::Unknown => {
                                    COLOR_DANGER.to_owned()
                                }
                                MonitorStatus::Warn => COLOR_WARNING.to_owned(),
                                MonitorStatus::NoData | MonitorStatus::Ignored => {
                                    COLOR_OTHER.to_owned()
                                }
                            },
                            status: match &status {
                                MonitorStatus::Ignored => "Ignored".to_owned(),
                                MonitorStatus::Skipped => "Skipped".to_owned(),
                                MonitorStatus::Ok => "Ok".to_owned(),
                                MonitorStatus::Alert => "Alert".to_owned(),
                                MonitorStatus::Unknown => "Unknown".to_owned(),
                                MonitorStatus::Warn => "Warn".to_owned(),
                                MonitorStatus::NoData => "No Data".to_owned(),
                            },
                            muted: !value.options.silenced.is_empty(),
                        },
                        200,
                    )
                } else {
                    (
                        BadgeOptions {
                            status: response.status().as_str().to_owned(),
                            color: COLOR_WARNING.to_owned(),
                            ..BadgeOptions::default()
                        },
                        if status_codes {
                            response.status().as_u16()
                        } else {
                            200
                        },
                    )
                }
            }
        }
    } else {
        (
            BadgeOptions {
                status: format!("Unconfigured account: {}", account),
                color: COLOR_OTHER.to_owned(),
                ..BadgeOptions::default()
            },
            if status_codes { 404 } else { 200 },
        )
    };
    {
        let mut cache = BADGE_CACHE.lock().unwrap();
        cache.cache_set(key, value.clone())
    }
    let (options, status_code) = value;
    Response::builder()
        .status(status_code)
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", format!("public,max-age={}", max_age))
        .body(Badge::new(options).to_svg())
        .map_err(|_| not_found())
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
    println!();
}

#[tokio::main]
async fn main() {
    let _ = ctrlc::set_handler(|| {
        info!("Stopped");
        exit(0)
    });
    env_logger::from_env(Env::default().default_filter_or("info,access=info")).init();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu and exit");
    opts.optflag("V", "version", "print the version and exit");
    opts.optopt(
        "",
        "host",
        "the host name to bind to (default: 0.0.0.0)",
        "HOST",
    );
    opts.optopt("", "port", "the port to bind to (default: 8080)", "PORT");
    opts.optopt(
        "",
        "context-root",
        "the context root to serve from (default: /)",
        "ROOT",
    );
    opts.optflag(
        "",
        "always-ok",
        "Always return images with status code HTTP/200",
    );

    // set up to parse the command line options
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    // process and validate the command line options
    if matches.opt_present("h") {
        print_usage(&program, &opts);
        return;
    }
    if matches.opt_present("V") {
        println!("{}", VERSION);
        return;
    }
    let host_port = format!(
        "{}:{}",
        matches
            .opt_get("host")
            .unwrap_or(None)
            .unwrap_or_else(|| "0.0.0.0".to_owned()),
        matches.opt_get_default("port", 8080).unwrap()
    );
    let status_codes = !matches.opt_present("always-ok");

    let log = warp::log("access");
    let monitor_badge = warp::path("accounts")
        .and(warp::path::param())
        .and(warp::path("monitors"))
        .and(warp::path::param())
        .and(warp::query::query())
        .and_then(move |account, id, query| get_monitor_badge(status_codes, account, id, query));
    let fallback = warp::any().map(|| {
        Response::builder()
            .status(404)
            .header("Content-Type", "text/html; charset=UTF-8")
            .body(include_str!("404.html"))
    });
    info!(
        "Listening for connections on {}/{}",
        host_port,
        matches
            .opt_default("context-root", "/")
            .unwrap_or_else(|| "/".to_owned())
    );

    let root = matches
        .opt_default("context-root", "/")
        .unwrap_or_else(|| "/".to_owned());
    if root != "/" && root != "" {
        warp::serve(warp::path(root).and(monitor_badge).or(fallback).with(log))
            .run(
                host_port
                    .as_str()
                    .to_socket_addrs()
                    .unwrap()
                    .next()
                    .unwrap(),
            )
            .await;
    } else {
        warp::serve(monitor_badge.or(fallback).with(log))
            .run(
                host_port
                    .as_str()
                    .to_socket_addrs()
                    .unwrap()
                    .next()
                    .unwrap(),
            )
            .await;
    }
}

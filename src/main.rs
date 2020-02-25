extern crate datadog_badges;

use std::env;
use std::net::ToSocketAddrs;
use std::process::exit;

use chrono::Utc;
use getopts::Options;
use regex::Regex;
use warp::reject::not_found;
use warp::{http::Response, Filter, Rejection};

use datadog_badges::badge::{
    Badge, BadgeOptions, COLOR_DANGER, COLOR_OTHER, COLOR_SUCCESS, COLOR_WARNING,
};
use datadog_badges::datadog::{get_monitor_details, MonitorState};

fn error_badge(status: u16, message: String) -> Result<Response<String>, Rejection> {
    Response::builder()
        .status(status)
        .header("Content-Type", "image/svg+xml")
        .header("Cache-Control", "public,max-age=15")
        .body(
            Badge::new(BadgeOptions {
                duration: None,
                status: message.to_owned(),
                color: COLOR_DANGER.to_owned(),
                ..BadgeOptions::default()
            })
            .to_svg(),
        )
        .map_err(|_| not_found())
}

async fn get_badge(account: String, id: String) -> Result<Response<String>, Rejection> {
    let client = reqwest::Client::new();
    let env_root = account.to_string().to_uppercase();
    let env_root = Regex::new(r"[^A-Z0-9_]")
        .unwrap()
        .replace_all(&env_root, "_");
    let app_key = env::var(format!("{}_APP_KEY", env_root));
    let api_key = env::var(format!("{}_API_KEY", env_root));
    if let (Ok(api_key), Ok(app_key)) = (api_key, app_key) {
        let details = get_monitor_details(&client, &api_key, &app_key, &id).await;
        match details {
            Err(_) => error_badge(500, "HTTP/500 Internal Server Error".to_owned()),
            Ok(response) => {
                if response.status().is_success() {
                    let value: MonitorState = response.json().await.map_err(|_| not_found())?;
                    Response::builder()
                        .header("Content-Type", "image/svg+xml")
                        .header("Cache-Control", "public,max-age=15")
                        .body(
                            Badge::new(BadgeOptions {
                                duration: match value.overall_state_modified {
                                    Some(v) => Some(Utc::now().signed_duration_since(v)),
                                    None => None,
                                },
                                color: match value.overall_state.to_uppercase().as_str() {
                                    "OK" => COLOR_SUCCESS.to_owned(),
                                    "ALERT" => COLOR_DANGER.to_owned(),
                                    "WARNING" => COLOR_WARNING.to_owned(),
                                    _ => COLOR_OTHER.to_owned(),
                                },
                                status: value.overall_state,
                                muted: !value.options.silenced.is_empty(),
                                ..BadgeOptions::default()
                            })
                            .to_svg(),
                        )
                        .map_err(|_| not_found())
                } else {
                    error_badge(
                        response.status().as_u16(),
                        response.status().as_str().to_owned(),
                    )
                }
            }
        }
    } else {
        error_badge(404, "HTTP/404 Not Found".to_owned())
    }
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options]", program);
    println!("{}", opts.usage(&brief));
    println!();
}

#[tokio::main]
async fn main() {
    let _ = ctrlc::set_handler(|| {
        println!("Stopped");
        exit(0)
    });
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu and exit");
    opts.optflag("V", "version", "print the version and exit");
    opts.optopt("", "host", "the host name to bind to (default: 0.0.0.0)", "HOST");
    opts.optopt("", "port", "the port to bind to (default: 8080)", "PORT");

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
            .unwrap_or("0.0.0.0".to_owned()),
        matches.opt_get_default("port", 8080).unwrap()
    );

    let badge = warp::path("account")
        .and(warp::path::param())
        .and(warp::path("monitors"))
        .and(warp::path::param())
        .and_then(get_badge);
    println!("Listening for connections on {}", host_port);
    warp::serve(badge)
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

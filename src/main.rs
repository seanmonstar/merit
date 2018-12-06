#[macro_use]
extern crate serde_derive;

use std::{env, fmt};

use futures::{future::{self, Either}, Future};
use reqwest::r#async::Client;
use warp::{http::{header, Response, StatusCode}, Filter, Reply};

#[cfg(test)] mod tests;

fn main() {
    pretty_env_logger::init();

    let port = get_port();
    println!("Binding to PORT {}", port);

    let routes = index()
        .or(badge());

    let log = warp::log("merit");

    warp::serve(routes.with(log))
        .run(([0, 0, 0, 0], port));
}

const DEFAULT_PORT: u16 = 8080;

fn get_port() -> u16 {
    match env::var("PORT") {
        Ok(val) => u16::from_str_radix(&val, 10).unwrap_or(DEFAULT_PORT),
        Err(_) => DEFAULT_PORT
    }
}

static INDEX_PAGE: &'static [u8] = include_bytes!("../assets/index.html");

fn index() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::get2()
        .and(warp::path::end())
        .map(|| {
            Response::builder()
                .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                .body(INDEX_PAGE)
        })
}

fn badge() -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    warp::get2()
        .and(style())
        .and(lookup())
        .map(|style: Option<Style>, lookup: Lookup| {
            let version = lookup.krate.max_version;

            let color = if version.starts_with("0") {
                "orange"
            } else {
                "brightgreen"
            };

            let style = match style {
                Some(Style::FlatSquare) => "?style=flat-square",
                None => "",
            };

            let badge = format!(
                "https://img.shields.io/badge/crates.io-v{}-{}.svg{}",
                ShieldEscape(&version),
                color,
                style
            );
            Response::builder()
                .status(StatusCode::FOUND)
                .header(header::LOCATION, badge)
                .header(header::EXPIRES, "Sun, 01 Jan 1990 00:00:00 GMT")
                .header(header::PRAGMA, "no-cache")
                .header(header::CACHE_CONTROL, "no-cache, no-store, max-age=0, must-revalidate")
                .body("")
        })
}

fn lookup() -> impl Filter<Extract = (Lookup,), Error = warp::Rejection> + Clone {
    let client = Client::new();
    warp::path::param()
        .and(warp::path::end())
        .and_then(move |crate_name: String| {
            let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
            client
                .get(&url)
                .header(header::USER_AGENT, "meritbadge/0.1")
                .send()
                .map_err(warp::reject::custom)
                .and_then(|mut api_res| {
                    match api_res.status() {
                        StatusCode::OK => {
                            let fut = api_res
                                .json()
                                .map_err(warp::reject::custom);
                            Either::A(fut)
                        },
                        StatusCode::NOT_FOUND => {
                            Either::B(future::err(warp::reject::not_found()))
                        },
                        _other => {
                            Either::B(future::err(warp::reject::not_found()))
                        }
                    }
                })
        })
}

fn style() -> impl Filter<Extract = (Option<Style>,), Error = warp::Rejection> + Clone {
    warp::query()
        .map(|q: Query| q.style)
}

#[derive(Deserialize)]
struct Lookup {
    #[serde(rename = "crate")]
    krate: Krate
}

#[derive(Deserialize)]
struct Krate {
    max_version: String
}

#[derive(Deserialize)]
struct Query {
    style: Option<Style>,
}

#[derive(Debug, Deserialize, PartialEq)]
enum Style {
    #[serde(rename = "flat-square")]
    FlatSquare,
}

struct ShieldEscape<'a>(&'a str);

impl<'a> fmt::Display for ShieldEscape<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::fmt::Write;
        for &byte in self.0.as_bytes() {
            if byte == b'-' {
                f.write_str("--")?;
            } else {
                f.write_char(byte as char)?;
            }
        }
        Ok(())
    }
}

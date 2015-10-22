extern crate hyper;
extern crate rustc_serialize;
extern crate env_logger;
extern crate time;

use std::env;
use std::io::{self, Read};
use std::str;

use rustc_serialize::json;
use time::Tm;

fn main() {
    env_logger::init().unwrap();
    let port = get_port();
    println!("Binding to PORT {}", port);
    let _listening = hyper::Server::http(("0.0.0.0", port)).unwrap()
        .handle(handle).unwrap();
    //println!("Listening on http://{}", listening.addr);
}

const DEFAULT_PORT: u16 = 8080;

fn get_port() -> u16 {
    match env::var("PORT") {
        Ok(val) => u16::from_str_radix(&val, 10).unwrap_or(DEFAULT_PORT),
        Err(_) => DEFAULT_PORT
    }
}

static INDEX_PAGE: &'static [u8] = include_bytes!("../assets/index.html");

const JAN_1990: Tm = Tm {
    tm_sec: 0,
    tm_min: 0,
    tm_hour: 0,
    tm_mday: 1,
    tm_mon: 0,
    tm_year: 90,
    tm_wday: 0,
    tm_yday: 0,
    tm_isdst: 0,
    tm_utcoff: 0,
    tm_nsec: 0,
};

fn handle(req: hyper::server::Request, mut res: hyper::server::Response<hyper::net::Fresh>) {
    match (req.method, req.uri) {
        (hyper::Get, hyper::uri::RequestUri::AbsolutePath(ref path)) if path == "/" => {
            let _ = res.send(INDEX_PAGE);
        }
        (hyper::Head, hyper::uri::RequestUri::AbsolutePath(path)) |
        (hyper::Get, hyper::uri::RequestUri::AbsolutePath(path)) => {
            let (crate_name, as_json) = if path.ends_with(".json") {
                (&path[1..(path.len() - 5)], true)
            } else {
                (&path[1..], false)
            };
            let version = match lookup(crate_name) {
                Ok(v) => v,
                Err(..) => return not_found(res)
            };
            if as_json {
                let msg = format!(r#"{{"version":"{}"}}"#, version);
                let _ = res.send(msg.as_ref());
            } else {
                let color = if version.as_bytes()[0] == b'0' {
                    "orange"
                } else {
                    "brightgreen"
                };

                let style = if path.find("?style=flat-square").is_some() {
                    "?style=flat-square"
                } else {
                    ""
                };
                let badge = format!(
                    "https://img.shields.io/badge/crates.io-v{}-{}.svg{}",
                    version,
                    color,
                    style
                );

                *res.status_mut() = hyper::status::StatusCode::Found;
                res.headers_mut().set(hyper::header::Location(badge));
                res.headers_mut().set(hyper::header::Expires(
                    hyper::header::HttpDate(JAN_1990)
                ));
                res.headers_mut().set(hyper::header::Pragma::NoCache);
                res.headers_mut().set(hyper::header::CacheControl(vec![
                    hyper::header::CacheDirective::NoCache,
                    hyper::header::CacheDirective::NoStore,
                    hyper::header::CacheDirective::MaxAge(0),
                    hyper::header::CacheDirective::MustRevalidate,
                ]));
            }
        },
        _ => {
            *res.status_mut() = hyper::status::StatusCode::MethodNotAllowed;
        }
    };
}

fn not_found(mut res: hyper::server::Response<hyper::net::Fresh>) {
    *res.status_mut() = hyper::NotFound;
    let _ = res.start().and_then(|res| res.end());
}

type LookupResult = Result<String, LookupError>;

enum LookupError {
    NotFound,
    Http(hyper::Error),
    Io(io::Error)
}

impl From<hyper::Error> for LookupError {
    fn from(e: hyper::Error) -> LookupError {
        LookupError::Http(e)
    }
}


impl From<io::Error> for LookupError {
    fn from(e: io::Error) -> LookupError {
        LookupError::Io(e)
    }
}

#[derive(Debug)]
struct LookupCrate {
    version: String
}

impl rustc_serialize::Decodable for LookupCrate {
    fn decode<D: rustc_serialize::Decoder>(d: &mut D) -> Result<LookupCrate, D::Error> {
        Ok(LookupCrate {
            version: try!(d.read_struct("", 1, |d| {
                d.read_struct_field("crate", 1, |d| {
                    d.read_struct("", 1, |d| {
                        d.read_struct_field("max_version", 0, |d| d.read_str())
                    })
                })
            }))
        })
    }
}
fn lookup(krate: &str) -> LookupResult {
    let client = hyper::Client::new();
    let url = format!("https://crates.io/api/v1/crates/{}", krate);
    let mut res = try!(client.get(&*url).send());
    if res.status != hyper::Ok {
        return Err(LookupError::NotFound);
    }

    let mut buf = Vec::new();
    try!(res.read_to_end(&mut buf));

    let text = match str::from_utf8(&buf) {
        Ok(text) => text,
        _ => return Err(LookupError::NotFound)
    };

    let map: LookupCrate = match json::decode(text) {
        Ok(map) => map,
        _ => return Err(LookupError::NotFound)
    };

    Ok(map.version)
}

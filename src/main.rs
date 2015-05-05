extern crate hyper;
extern crate rustc_serialize;
extern crate env_logger;
extern crate time;

use std::env;
use std::io::{self, Read, Write};
use std::str;

use rustc_serialize::json;
use time::Tm;

fn main() {
    env_logger::init().unwrap();
    let port = get_port();
    println!("Binding to PORT {}", port);
    let listening = hyper::Server::new(handle)
        .listen(("0.0.0.0", port)).unwrap();
    println!("Listening on http://{}", listening.socket);
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
            res.headers_mut().set(hyper::header::ContentLength(INDEX_PAGE.len() as u64));
            let _ = res.start().and_then(|mut res| {
                let _ = res.write_all(INDEX_PAGE);
                res.end()
            });
        }
        (hyper::Head, hyper::uri::RequestUri::AbsolutePath(path)) |
        (hyper::Get, hyper::uri::RequestUri::AbsolutePath(path)) => {
            let version = match lookup(&path[1..]) {
                Ok(v) => v,
                Err(..) => return not_found(res)
            };
            let color = if v[0] == b'0' {
                "orange"
            } else {
                "brightgreen"
            };
            let badge = format!(
                "https://img.shields.io/badge/crates.io-v{}-{}.svg",
                version,
                color
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
            let _ = res.start().and_then(|res| res.end());
        },
        _ => {
            *res.status_mut() = hyper::status::StatusCode::MethodNotAllowed;
            let _ = res.start().and_then(|res| res.end());
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
    Http(hyper::HttpError),
    Io(io::Error)
}

impl From<hyper::HttpError> for LookupError {
    fn from(e: hyper::HttpError) -> LookupError {
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
    let mut client = hyper::Client::new();
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

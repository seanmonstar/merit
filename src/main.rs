extern crate hyper;
extern crate rustc_serialize;
extern crate env_logger;

use std::env;
use std::io::{self, Read};
use std::str;

use rustc_serialize::json;

fn main() {
    env_logger::init();
    let port = get_server_port();
    let _l = hyper::Server::new(handle).listen(("127.0.0.1", port));
    println!("Listening on http://127.0.0.1:{}", port);
}

const DEFAULT_PORT: u16 = 8080;

fn get_server_port() -> u16 {
    match env::var("PORT") {
        Ok(val) => u16::from_str_radix(&val, 10).unwrap_or(DEFAULT_PORT),
        Err(_) => DEFAULT_PORT
    }
}
fn handle(req: hyper::server::Request, mut res: hyper::server::Response<hyper::net::Fresh>) {
    match (req.method, req.uri) {
        (hyper::Get, hyper::uri::RequestUri::AbsolutePath(path)) => {
            let version = match lookup(&path[1..]) {
                Ok(v) => v,
                Err(..) => return not_found(res)
            };
            let badge = format!("https://img.shields.io/badge/crates.io-{}-green.svg", version);
            *res.status_mut() = hyper::status::StatusCode::Found;
            res.headers_mut().set(hyper::header::Location(badge));
            let _ = res.start().and_then(|res| res.end());
        },
        _ => not_found(res)
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

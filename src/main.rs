extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde;
extern crate serde_json;
extern crate hyper_tls;

#[macro_use]
extern crate serde_derive;

use std::str;
use std::fs::File;
use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::{Request, Client, Method, Chunk};
use hyper::header::{Basic, Authorization, ContentLength, ContentType};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;

#[derive(Deserialize)]
struct SecretsConfig {
    spotify_client_id: String,
    spotify_secret: String,
    //All these do now is give me compiler warnings...
    //discord_client_id: String,
    //discord_secret: String
}

#[derive(Deserialize)]
struct SpotifyAuthResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

const SPOTIFY_AUTH_ENDPOINT: &'static str = "https://accounts.spotify.com/api/token";
const SPOTIFY_AUTH_REQUEST_BODY: &'static str = "grant_type=client_credentials";

const SPOTIFY_SEARCH_ENDPOINT: &'static str = "https://api.spotify.com/v1/search";

fn chunk_to_bytes(chunk: Chunk) -> Vec<u8> {
    println!("chunk-to-bytes!");
    chunk.into_iter().collect()
}

fn spotify_auth(config: SecretsConfig) -> SpotifyAuthResponse {
    //Create client
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    //Build request
    let mut req = Request::new(Method::Post, SPOTIFY_AUTH_ENDPOINT.parse().unwrap());
    req.headers_mut().set(Authorization(Basic {
        username: config.spotify_client_id,
        password: Some(config.spotify_secret),
    }));
    req.headers_mut().set(ContentLength(
        SPOTIFY_AUTH_REQUEST_BODY.len() as u64,
    ));
    req.headers_mut().set(ContentType::form_url_encoded());
    req.set_body(SPOTIFY_AUTH_REQUEST_BODY);

    let work = client
        .request(req)
        .and_then(|resp| resp.body().map(chunk_to_bytes).collect())
        .and_then(|v| Ok(v.iter().flat_map(|x| x.clone()).collect::<Vec<_>>()));

    let result = core.run(work).unwrap();
    serde_json::from_str::<SpotifyAuthResponse>(str::from_utf8(&result).unwrap()).unwrap()
}

fn main() {
    //Read API keys from config file
    let secrets_config_file = File::open("secrets.json").unwrap();
    let config: SecretsConfig = serde_json::from_reader(secrets_config_file).unwrap();
    println!("spotify client id = {}", config.spotify_client_id);

    println!("{}", spotify_auth(config).expires_in);
}

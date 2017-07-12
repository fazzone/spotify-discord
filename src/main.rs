extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde;
extern crate serde_json;
extern crate hyper_tls;
extern crate url;

#[macro_use]
extern crate serde_derive;

use std::str;
use std::fs::File;
use std::io::{self, Error, Write};
use futures::{Future, Stream};
use hyper::{Request, Client, Method, Chunk};
use hyper::client::HttpConnector;
use hyper::header::{Authorization, Basic, Bearer, ContentLength, ContentType};
use hyper_tls::HttpsConnector;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};
use tokio_core::reactor::{Core, Handle};

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

type SpotifyAuthToken = String;

const SPOTIFY_AUTH_ENDPOINT: &'static str = "https://accounts.spotify.com/api/token";
const SPOTIFY_AUTH_REQUEST_BODY: &'static str = "grant_type=client_credentials";

fn chunk_to_bytes(chunk: Chunk) -> Vec<u8> {
    //do we really need to collect() here and in the calling code?
    chunk.into_iter().collect()
}

//Give Spotify our client ID and secret to get a bearer auth token that we can
//use for actual API requests
fn get_auth_token<T: hyper::client::Connect>(
    client: &Client<T>,
    config: SecretsConfig,
) -> Box<Future<Item = SpotifyAuthToken, Error = String>> {

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

    Box::new(
        client
            .request(req)
            .and_then(|resp| resp.body().map(chunk_to_bytes).collect())
            .and_then(|v| Ok(v.iter().flat_map(|x| x.clone()).collect::<Vec<_>>()))
            .map_err(|e| e.to_string())
            .and_then(|r| match serde_json::from_str::<SpotifyAuthResponse>(
                str::from_utf8(&r).map_err(|e| e.to_string())?,
            ) {
                Ok(v) => Ok(v.access_token),
                Err(e) => Err(e.to_string()),
            }),
    )
}

#[derive(Debug)]
struct SpotifyTrack {
    name: String,
    id: String,
}

fn search_for_track<T: hyper::client::Connect>(
    client: &Client<T>,
    auth_token: SpotifyAuthToken,
    search_type: String,
    search_str: String,
) -> Box<Future<Item = SpotifyTrack, Error = String>> {

    let search_url = format!(
        "https://api.spotify.com/v1/search?q={}&type={}&limit=1",
        utf8_percent_encode(search_str.as_ref(), DEFAULT_ENCODE_SET).to_string(),
        search_type,
    ).parse()
        .unwrap();
    let mut req = Request::new(Method::Get, search_url);
    req.headers_mut().set(
        Authorization(Bearer { token: auth_token }),
    );
    req.headers_mut().set(ContentType::form_url_encoded());

    Box::new(
        client
            .request(req)
            .and_then(|resp| resp.body().map(chunk_to_bytes).collect())
            .and_then(|v| Ok(v.iter().flat_map(|x| x.clone()).collect::<Vec<_>>()))
            .map_err(|e| e.to_string())
            .and_then(|bs| match serde_json::from_slice(&bs) {
                Ok(json) => Ok(json),
                Err(e) => Err(e.to_string()),
            })
            .and_then(|v: serde_json::Value| {
                let ref track_info = v["tracks"]["items"][0];
                let name_val = track_info["name"].clone();
                let id_val = track_info["id"].clone();
                match (name_val, id_val) {
                    (serde_json::Value::String(name), serde_json::Value::String(id)) => {
                        Ok(SpotifyTrack { name: name, id: id })
                    }
                    _ => Err(From::from("no results?")),
                }
            }),
    )
}

fn main() {
    //Create client
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle).unwrap())
        .build(&handle);

    //Read API keys from config file
    let secrets_config_file = File::open("secrets.json").unwrap();
    let config: SecretsConfig = serde_json::from_reader(secrets_config_file).unwrap();

    let auth_token = core.run(get_auth_token(&client, config)).unwrap();
    println!(
        "{:?}",
        core.run(search_for_track(
            &client,
            auth_token,
            "track".to_owned(),
            "wool in the wash".to_owned(),
        )).unwrap()
    );

}

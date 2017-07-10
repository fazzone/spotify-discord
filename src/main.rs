extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

#[derive(Deserialize)]
struct SecretsConfig {
    spotify_client_id: String,
    spotify_secret: String,
    //All these do now is give me compiler warnings... 
    //discord_client_id: String,
    //discord_secret: String
}

fn main() {
    //HTTP request example code...
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let uri = "http://ifconfig.io".parse().unwrap();
    let work = client.get(uri).and_then(|res| {
        println!("status {}", res.status());
        res.body().for_each(|chunk| {
            io::stdout().write_all(&chunk).map_err(From::from)
        })
    });;
    core.run(work).unwrap();

    println!("================================================================");

    //Read API keys from config file
    let secrets_config_file = File::open("secrets.json").unwrap();
    let config: SecretsConfig = serde_json::from_reader(secrets_config_file).unwrap();
    println!("spotify client id = {}", config.spotify_client_id);
}

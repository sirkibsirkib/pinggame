
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate mio;
extern crate mio_extras;
extern crate middleman;
extern crate clap;
extern crate rand;

use clap::App;
use middleman::{
	Middleman,
	Message,
	PackedMessage,
};

use std::{
	net::SocketAddr,
};

mod game;
use game::*;

mod common;
use common::*;


mod server;
mod client;

fn main() {
	let matches = App::new("Pinggame")
	        .version("1.0")
	        .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
	        .about("A super small rust server client toy game for testing network RTT.")
	        .args_from_usage("-m, --moniker=[CHAR] 'Choose a character-moniker for this game session. eg: `$`'
	                         <ip> 'Sets the bind/connect addr'")
	        .get_matches();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(ip) = matches.value_of("ip") {
        println!("Value for server: {}", ip);
    }

    let ip = matches.value_of("ip").unwrap();
	if let Ok(addr) = ip.parse::<SocketAddr>() {
		println!("ADDR {:?}", &addr);
		match matches.value_of("moniker") {
	    	Some(moniker) => {
	    		if moniker.len() != 1 {
	    			println!("You need to provide a 1-char moniker!");
	    			return;
	    		}
	    		let m = Moniker(moniker.chars().next().unwrap());
	    		println!("Welcome, player `{}`.", m.0);
				client::client_enter(&addr, m);
	    	},
	    	None => server::server_enter(&addr),
	    };
	} else {
		println!("Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
	}
}


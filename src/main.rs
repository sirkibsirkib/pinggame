
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate mio;
extern crate mio_extras;
extern crate middleman;
extern crate clap;

use clap::App;
use middleman::{
	Middleman,
	Message,
};

use std::{
	net::SocketAddr,
};

mod game;
use game::*;

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
		match matches.value_of("name") {
	    	Some(name) => {
	    		if name.len() != 1 {
	    			println!("You need to provide a 1-char moniker!");
	    			return;
	    		}
				client(&addr, Moniker(name.chars().next().unwrap()) );
	    	},
	    	None => server(&addr),
	    };
	} else {
		println!("Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
	}
}


////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientWard {
	Welcome(State),
	UpdMove(Moniker, Direction),
	ErrorTakenMoniker,
	ErrorIllegalMove,
}
impl middleman::Message for ClientWard {}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Serverward {
	Hello(Moniker),
	ReqMove(Direction),
}
impl middleman::Message for Serverward {}
////////////////////////////////////////////////////////////



fn client(addr: &SocketAddr, moniker: Moniker) {
	println!("Client starting, for server at addr {:?}!", addr);

}


struct ServerState {

}
fn server(addr: &SocketAddr) {
	println!("Server starting at arrd {:?}!", addr);
}

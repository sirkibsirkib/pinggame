
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
};

use std::{
	net::SocketAddr,
};
use mio::{
	Poll,
	Ready,
	PollOpt,
	Events,
	Token,
};

mod game;
use game::*;

type MioStream = mio::net::TcpStream;
type StdStream = std::net::TcpStream;
type MioListener = mio::net::TcpListener;

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
				client(&addr, m);
	    	},
	    	None => server(&addr),
	    };
	} else {
		println!("Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
	}
}


////////////////////////////////////////////////////////////
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Clientward {
	Welcome(GameState, Coord2D),
	UpdMove(Moniker, Direction),
	ErrorTakenMoniker,
	ErrorIllegalMove,
	ErrorSocketDead,
}
impl middleman::Message for Clientward {}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Serverward {
	Hello(Moniker),
	ReqMove(Direction),
}
impl middleman::Message for Serverward {}
////////////////////////////////////////////////////////////



fn client(addr: &SocketAddr, moniker: Moniker) {
	println!("Client starting, for server at addr {:?}!", addr);
	match StdStream::connect(addr) {
		Ok(stream) => {
			// TODO
			println!("Connected ok!");
		},
		Err(e) => {
			println!("Failed to connect to addr `{:?}`. Got err {:?}", addr, e);
		}
	}
}

type Clients = HashMap<Token, ClientObject>;
type Newcomers = HashMap<Token, Middleman>;

struct ClientObject {
	middleman: Middleman,
	moniker: Moniker,
}

use std::collections::HashMap;
const LISTENER_TOKEN: Token = Token(0);



fn server(addr: &SocketAddr) {
	println!("Server starting at addr {:?}!", addr);
	let listener = MioListener::bind(addr)
		.expect("Failed to bind");

	let poll = Poll::new().unwrap();
	let mut events = Events::with_capacity(128);
    poll.register(&listener, LISTENER_TOKEN, Ready::readable(), PollOpt::edge()).unwrap();
    let mut clients: Clients = HashMap::new();
	let mut newcomers: HashMap<Token, Middleman> = HashMap::new();
	let mut server_control: Vec<ServerCtrlMsg> = vec![];
	let mut game_state = GameState::new();
    loop {
    	poll.poll(&mut events, None).unwrap();
    	for event in events.iter() {
    		match event.token() {
    			LISTENER_TOKEN => {
    				// LISTENER ACCEPT
    				match listener.accept() {
						Ok((stream, _addr)) => {
							let mm = Middleman::new(stream);
				    		let tok = next_free_token(&clients, &newcomers);
				    		poll.register(&mm, tok,
						    			Ready::writable() | Ready::readable(),
						    			PollOpt::oneshot()).unwrap();
				    		newcomers.insert(tok, mm);
						},
						Err(e) => panic!("DEAD BOI"), // TODO
					}
    			},
    			tok => {
    				if clients.contains_key(&tok) {
    					handle_client_incoming(&mut clients, tok, &mut server_control)
    				} else if newcomers.contains_key(&tok) {
    					handle_newcomer_incoming(&mut newcomers, tok, &mut server_control);
    				} else {
    					panic!("WHOSE TOKEN??");
    				}
    			},
    		}
    	}

    	if !server_control.is_empty() {
    		do_server_control(&mut server_control, &mut newcomers,
    			              &mut clients, &poll, &mut game_state);
    	}
    }
}


enum ServerCtrlMsg {
	DropNewcomerWithErr(Token, Clientward),
	DropClientWithErr(Token, Clientward),
	UpgradeClient(Token, Moniker),
}
fn do_server_control(server_control: &mut Vec<ServerCtrlMsg>, newcomers: &mut Newcomers,
	                 clients: &mut Clients, poll: &Poll, game_state: &mut GameState)
{
	for ctrl_msg in server_control.drain(..) {
		match ctrl_msg {
			ServerCtrlMsg::DropNewcomerWithErr(tok, msg) => {

			},
			ServerCtrlMsg::DropClientWithErr(tok, msg) => {

			},
			ServerCtrlMsg::UpgradeClient(tok, moniker) => {
				let mut mm = newcomers.remove(&tok).unwrap();
				if game_state.contains_moniker(moniker) {
					let _ = mm.send(& Clientward::ErrorTakenMoniker);
				} else {
					let coord = game_state.random_free_spot().expect("GAME TOO FULL");
					if game_state.try_put_moniker(moniker, coord) {
						if mm.send(& Clientward::Welcome(game_state.clone() , coord)).is_ok() {
							poll.register(&mm, tok,
						    			Ready::readable(),
						    			PollOpt::edge()).unwrap();
				    		let x = ClientObject {
				    			moniker: moniker,
				    			middleman: mm,
				    		};
				    		clients.insert(tok, x);
						}
					}
				}
			},
		}
	}
}


fn handle_client_incoming(clients: &mut Clients, tok: Token, server_control: &mut Vec<ServerCtrlMsg>) {

}

fn handle_newcomer_incoming(newcomers: &mut Newcomers, tok: Token, server_control: &mut Vec<ServerCtrlMsg>) {
	use ServerCtrlMsg::UpgradeClient;
	let mut done = false;
	let mm: &mut Middleman = newcomers.get_mut(&tok).expect("newcomer incoming");
	mm.recv_all_map( |_me, msg| {
		if let Serverward::Hello(moniker) = msg {
			if !done {
				server_control.push(UpgradeClient(tok, moniker));
				done = true;
			}
		}
	});
}

fn next_free_token(c: &Clients, n: &Newcomers) -> Token {
	for x in 1.. {
		let tok = Token(x);
		if c.contains_key(&tok)
		|| n.contains_key(&tok) {
			continue;
		}
		return tok
	}
	panic!("No available tokens!")
}
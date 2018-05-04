
use ::common::*;
use ::game::*;

use middleman::{
	Middleman,
	Message,
	PackedMessage,
};

use std::{
	net::SocketAddr,
	collections::HashMap,
};
use mio::{
	Poll,
	Ready,
	PollOpt,
	Events,
	Token,
};


const LISTENER_TOKEN: Token = Token(0);

type Clients = HashMap<Token, ClientObject>;
type Newcomers = HashMap<Token, Middleman>;

struct ClientObject {
	middleman: Middleman,
	moniker: Moniker,
}

enum ServerCtrlMsg {
	DropNewcomerWithErr(Token, Clientward),
	DropClientWithErr(Token, Clientward),
	UpgradeClient(Token, Moniker),
}

pub fn server_enter(addr: &SocketAddr) {
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
	let mut outgoing_updates = vec![];
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
    					handle_client_incoming(&mut clients, tok, &mut server_control,
    						                   &mut game_state, &mut outgoing_updates);
    				} else if newcomers.contains_key(&tok) {
    					handle_newcomer_incoming(&mut newcomers, tok, &mut server_control);
    				} else {
    					panic!("WHOSE TOKEN??");
    				}
    			},
    		}
    	}

    	if !outgoing_updates.is_empty() {
    		broadcast_outgoing_updates(&mut outgoing_updates, &mut clients, &mut server_control);
    	}

    	if !server_control.is_empty() {
    		do_server_control(&mut server_control, &mut newcomers, &mut clients, &poll,
    			              &mut game_state, &mut outgoing_updates);
    	}
    }
}

#[inline]
fn broadcast_outgoing_updates(outgoing_updates: &mut Vec<Clientward>, clients: &mut Clients,
	                          server_control:  &mut Vec<ServerCtrlMsg>)
{
	use self::ServerCtrlMsg::*;
	for msg in outgoing_updates.drain(..) {
		let packed = PackedMessage::new(& msg).expect("failed to pack");
		for (&tok, client_object) in clients.iter_mut() {
			if client_object.middleman.send_packed(& packed).is_err() {
				server_control.push(DropClientWithErr(tok, Clientward::ErrorSocketDead));
			}
		}
	}
}

#[inline]
fn do_server_control(server_control: &mut Vec<ServerCtrlMsg>, newcomers: &mut Newcomers,
	                 clients: &mut Clients, poll: &Poll, game_state: &mut GameState,
	                 outgoing_updates: &mut Vec<Clientward>)
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
						if mm.send(& Clientward::Welcome(game_state.clone() , moniker)).is_ok() {
							poll.register(&mm, tok,
						    			Ready::readable(),
						    			PollOpt::edge()).unwrap();
				    		let x = ClientObject {
				    			moniker: moniker,
				    			middleman: mm,
				    		};
				    		clients.insert(tok, x);
				    		outgoing_updates.push(Clientward::AddPlayer(moniker, coord));
						}
					}
				}
			},
		}
	}
}

#[inline]
fn handle_client_incoming(clients: &mut Clients, tok: Token, server_control: &mut Vec<ServerCtrlMsg>,
	                      game_state: &mut GameState, outgoing_updates: &mut Vec<Clientward>)
{
	use self::ServerCtrlMsg::*;
	let client_object = clients.get_mut(&tok).expect("clients incoming");
	let moniker = client_object.moniker;
	loop {
		match client_object.middleman.recv::<Serverward>() {
			Ok(Some(Serverward::ReqMove(dir))) => {
				if game_state.move_moniker_in_dir(moniker, dir) {
					outgoing_updates.push(Clientward::UpdMove(moniker, dir))
				}
			},
			Ok(None) => (), // spurious wakeup
			Ok(Some(msg)) => {
				server_control.push(DropClientWithErr(tok, Clientward::ErrorExpectedReq));
			}
			Err(e) => {
				server_control.push(DropClientWithErr(tok, Clientward::ErrorSocketDead));
			},
		}
	}
}

#[inline]
fn handle_newcomer_incoming(newcomers: &mut Newcomers, tok: Token, server_control: &mut Vec<ServerCtrlMsg>) {
	use self::ServerCtrlMsg::*;
	let mut done = false; // drop all but first message
	let mm: &mut Middleman = newcomers.get_mut(&tok).expect("newcomer incoming");
	mm.recv_all_map( |_me, msg| {
		if done { return }
		if let Serverward::Hello(moniker) = msg {
			server_control.push(UpgradeClient(tok, moniker));
		} else {
			server_control.push(DropNewcomerWithErr(tok, Clientward::ErrorExpectedHello));
		}
		done = true;
	});
}

#[inline]
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
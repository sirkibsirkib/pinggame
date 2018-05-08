
use ::common::*;
use ::game::*;

use ::rand::{self,
	Rng,
};

use middleman::{
	Middleman,
	PackedMessage,
};

use std::{
	net::SocketAddr,
	collections::HashMap,
	time::{
		Duration,
		Instant,
	},
};
use mio::{
	Poll,
	Ready,
	PollOpt,
	Events,
	Token,
};

const LISTENER_TOKEN: Token = Token(0);
const SERVER_SLEEP_TIME: Duration = Duration::from_millis(200);

type Clients = HashMap<Token, ClientObject>;
type Newcomers = HashMap<Token, Middleman>;

#[derive(Debug)]
struct ClientObject {
	middleman: Middleman,
	moniker: Moniker,
	last_move_at: Instant,
}

#[derive(Clone, Debug)]
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
	let mut events = Events::with_capacity(256);
    poll.register(&listener, LISTENER_TOKEN, Ready::readable(), PollOpt::edge()).unwrap();
    let mut clients: Clients = HashMap::new();
	let mut newcomers: HashMap<Token, Middleman> = HashMap::new();
	let mut server_control: Vec<ServerCtrlMsg> = vec![];
	let mut game_state = GameState::new_random();
	let mut bots = init_bots(&mut game_state);

	let mut outgoing_updates = vec![];
    loop {
    	let poll_sleep = if clients.is_empty() { None } else { Some(SERVER_SLEEP_TIME) };
    	poll.poll(&mut events, poll_sleep).unwrap();
    	// ::std::thread::sleep(Duration::from_millis(200));
    	for event in events.iter() {
    		match event.token() {
    			LISTENER_TOKEN => {
    				// LISTENER ACCEPT
    				match listener.accept() {
						Ok((stream, _addr)) => {
							stream.set_nodelay(true).expect("set_nodelay call failed");
							let mm = Middleman::new(stream);
				    		let tok = next_free_token(&clients, &newcomers);
							println!("Newcomer client with {:?}", tok);
				    		poll.register(&mm, tok,
						    			Ready::readable(),
						    			PollOpt::edge()).unwrap();
				    		newcomers.insert(tok, mm);
						},
						Err(e) => {
							println!("Listener died! {}", e);
							panic!("Listener died");
						},
					}
    			},
    			tok => {
    				if !event.readiness().is_readable() {
    					continue;
    				}
    				if clients.contains_key(&tok) {
    					// println!("...client");
    					handle_client_incoming(&mut clients, tok, &mut server_control,
    						                   &mut game_state, &mut outgoing_updates);
    				} else if newcomers.contains_key(&tok) {
    					// println!("...newcomer");
    					handle_newcomer_incoming(&mut newcomers, tok, &mut server_control);
    				} else {
    					panic!("WHOSE TOKEN??");
    				}
    			},
    		}
    	}

    	if !clients.is_empty() {
    		// freeze the game when there are no clients
    		game_tick(&mut game_state, &mut outgoing_updates, &mut bots);
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
		// println!("broadcasting {:?}", &msg);
		let packed = PackedMessage::new(& msg).expect("failed to pack");
		match msg {
			Clientward::AddPlayer(moniker, _coord) => {
				for (&tok, client_object) in clients.iter_mut() {
					if client_object.moniker == moniker {
						continue; // no need to add yourself.
					}
					if client_object.middleman.send_packed(& packed).is_err() {
						server_control.push(DropClientWithErr(tok, Clientward::ErrorSocketDead));
					}
				}
			},
			_ => {
				for (&tok, client_object) in clients.iter_mut() {
					if client_object.middleman.send_packed(& packed).is_err() {
						server_control.push(DropClientWithErr(tok, Clientward::ErrorSocketDead));
					}
				}
			},
		} 
		
	}
}

struct Bot {
	moniker: Moniker,
	last_move_at: Instant,
}

fn init_bots(game_state: &mut GameState) -> Vec<Bot> {
	let mut bots = vec![];
	for bot_moniker in ['0', '1']
	.iter().map(|&c| Moniker(c))
	{
		let coord = game_state.random_free_spot().expect("No coord to put bot");
		if game_state.try_add_player(bot_moniker, coord) {
			bots.push(Bot { moniker: bot_moniker, last_move_at: Instant::now() });
		} else {
			panic!("Failed to place bot {:?}", bot_moniker);
		}
	}
	bots
}

fn game_tick(game_state: &mut GameState, outgoing_updates: &mut Vec<Clientward>,
	         bots: &mut Vec<Bot>)
{
	let mut rng = rand::thread_rng();
	for bot in bots.iter_mut() {
		if bot.last_move_at.elapsed() > BOT_MOVE_PERIOD {
			let moniker = bot.moniker;
			let dir = *rng.choose(&DIR_CHOICES).unwrap();
			if game_state.move_moniker_in_dir(moniker, dir) {
				bot.last_move_at = Instant::now();
				outgoing_updates.push(Clientward::UpdMove(moniker, dir))
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
		println!("handing control msg {:?}", &ctrl_msg);
		match ctrl_msg {
			ServerCtrlMsg::DropNewcomerWithErr(tok, msg) => {
				if let Some(mut mm) = newcomers.remove(&tok) {
					let _ = poll.deregister(&mm);
					let _ = mm.send(& msg);
				}
			},
			ServerCtrlMsg::DropClientWithErr(tok, msg) => {
				if let Some(mut obj) = clients.remove(&tok) {
					let _ = poll.deregister(& obj.middleman);
					let _ = obj.middleman.send(& msg);
					if game_state.try_remove_player(obj.moniker) {
						outgoing_updates.push(Clientward::RemovePlayer(obj.moniker));
					}
				}
			},
			ServerCtrlMsg::UpgradeClient(tok, moniker) => {
				let mut mm = newcomers.remove(&tok).expect("remove fail");
				if game_state.contains_player(moniker) {
					let _ = mm.send(& Clientward::ErrorTakenMoniker);
				} else {
					let coord = game_state.random_free_spot().expect("GAME TOO FULL");
					if game_state.try_add_player(moniker, coord) {
						if mm.send(& Clientward::Welcome(game_state.get_essence().clone())).is_ok() {
							poll.deregister(&mm).expect("deregister fail");
							poll.reregister(&mm, tok,
						    			Ready::readable() | Ready::writable(),
						    			PollOpt::edge()).expect("reregister fail");
				    		let x = ClientObject {
				    			moniker: moniker,
				    			middleman: mm,
				    			last_move_at: Instant::now(),
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
		let x = client_object.middleman.recv::<Serverward>();
		println!("got from client {:?} {:?}", tok, &x);
		match x {
			Ok(Some(Serverward::ReqMove(dir))) => {
				if client_object.last_move_at.elapsed() < MOVE_PERIOD {
					println!("tok {:?} is moving too fast", tok);
					continue; //moving too fast
				} 
				if game_state.move_moniker_in_dir(moniker, dir) {
					client_object.last_move_at = Instant::now();
					outgoing_updates.push(Clientward::UpdMove(moniker, dir))
				}
				// don't drop clients if they misbehave. just silently drop msg
			},
			Ok(None) => break, // spurious wakeup
			Ok(Some(_msg)) => {
				server_control.push(DropClientWithErr(tok, Clientward::ErrorExpectedReq));
				break;
			}
			Err(_e) => {
				server_control.push(DropClientWithErr(tok, Clientward::ErrorSocketDead));
				break;
			},
		}
	}
}

#[inline]
fn handle_newcomer_incoming(newcomers: &mut Newcomers, tok: Token, server_control: &mut Vec<ServerCtrlMsg>) {
	use self::ServerCtrlMsg::*;
	let mut done = false; // drop all but first message
	let mm: &mut Middleman = newcomers.get_mut(&tok).expect("newcomer incoming");
	if mm.recv_all_map( |_me, msg| {
		println!("got from newcomer {:?} {:?}", tok, &msg);
		if done { return }
		if let Serverward::Hello(moniker) = msg {
			server_control.push(UpgradeClient(tok, moniker));
		} else {
			server_control.push(DropNewcomerWithErr(tok, Clientward::ErrorExpectedHello));
		}
		done = true;
	}).1.is_err() {
		server_control.push(DropNewcomerWithErr(tok, Clientward::ErrorSocketDead));
	}
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
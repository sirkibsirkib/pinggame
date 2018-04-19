
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate mio;
extern crate middleman;
extern crate clap;

use clap::{App, SubCommand};
use middleman::{
	Middleman,
	Message,
	Threadless,
};
use std::collections::{
	HashSet,
};

use std::{
	env,
	thread,
};
use mio::*;
use std::net::SocketAddr;
use mio::tcp::{
	TcpListener,
	TcpStream,
};
use std::io::{
	Read,
	Write,
	ErrorKind,
};

fn main() {
	let matches = App::new("Pinggame")
                        .version("1.0")
                        .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
                        .about("A super small rust server client toy game for testing network RTT.")
                        .args_from_usage("-n, --name=[FILE] 'Set the name for a client session'
                                         <ip> 'Sets the bind/connect addr'")
                        .get_matches();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(ip) = matches.value_of("ip") {
        println!("Value for server: {}", ip);
    }

    // if let Some(ip) = matches.value_of("ip") {
    //     println!("Value for server: {}", ip);
    // }

	let addr: SocketAddr = matches.value_of("ip").expect("NO IP??").parse().unwrap();
	println!("ADDR {:?}", &addr);



    match matches.value_of("name") {
    	Some(name) => client(&addr, name.to_owned()),
    	None => server(&addr),
    };

    // match matches.occurrences_of("server") {
    //     0 => client(&addr).ok(),
    //     _ => server(&addr).ok(),
    // };
}

use std::sync::mpsc::{
	Receiver,
	Sender,
	channel,
};

fn client(addr: &SocketAddr, name: String) {
	println!("CLIENT START with name `{}`", &name);
	// let client_tok = Token(0);
	// let poll = Poll::new().unwrap();
	let sock = TcpStream::connect(&addr).unwrap();
	let mut mm = Threadless::new(sock);
	mm.send(& Serverward::Hello{ name: name });
	match mm.recv::<Clientward>() {
		Ok(Clientward::Welcome{ position: p}) => {
			client_play(p, mm)
		},
		x => {
			println!("CLIENT EXPECTED WELCOME {:?}", &x);
			return;
		},
	}
	
}

enum Directions { //TODO
	Left, Right,
}

fn client_play<M: Middleman>(pos: usize, mut mm: M) {
	println!("I have been given pos {:?}", pos);
	let mut game_state = GameState::new();
	let (mut s, mut r) = channel(); //channel between keylistener thread and main
	let handle = thread::spawn(move || {
		let mut buffer = String::new();
		println!("keylister going at it...");
		loop {
			println!("keylister reading line...");
			buffer.clear();
	    	std::io::stdin().read_line(&mut buffer).unwrap();
	    	match buffer.trim() {
	    		"l" => {
	    			s.send(true);//left
	    		},
	    		"r" => {
	    			s.send(false);//right
	    		},
	    		_ => {println!("read `{}`", &buffer);},
	    	}
		}
	});
	let sleepytime = std::time::Duration::from_millis(200);
	loop {
		//sleep
		thread::sleep(sleepytime);

		while let Ok(dir) = r.try_recv() {
			//request a move
			mm.send(& Serverward::MoveMe { left: dir } ).ok();
		}

		match mm.try_recv::<Clientward>() {
			Ok(msg) => {
				use Clientward::*;
				println!("Got {:?} from server...", msg);
				match msg {
					Welcome { position: p } => (), // wtf
					BadName => (), //wtf
					MovePiece { pos: p, left: l } => {
						println!("moving piece in pos {:?} to {}", p, if l {"left"} else {"right"} );
						game_state.move_piece(p, l);
						game_state.draw();
					},
					AddPlayer { start_h: s, name: n } => {
						game_state.add_player(s, n);
						game_state.draw();
					},
					RemovePlayer { pos: p } => {
						game_state.remove_player(pos);
						game_state.draw();
					},
					GameState { state: g } => {
						game_state = g;
						game_state.draw();
					}, 
				}
			},
			Err(middleman::TryRecvError::ReadNotReady) => (),
			Err(e) => {
				println!("Something went wrong");
				return;
			}
		}
	}
	//TODO
}


enum ClientState {
	ExpectingHello,
	Playing(usize),
}
struct ClientData<M: Middleman> {
	mm: M,
	state: ClientState,
}


// spawn an echo server on localhost. return the thread handle and the bound ip addr.
fn server(addr: &SocketAddr) {
	let listener = TcpListener::bind(addr).unwrap();
	let poll = Poll::new().unwrap();
	poll.register(&listener, Token(0), Ready::readable(),
		PollOpt::edge()).unwrap();
	let mut events = Events::with_capacity(128);
	let tick_millis = std::time::Duration::from_millis(300);

	let mut game_state = GameState::new();
	let mut client_data = vec![];
	let mut to_broadcast = vec![];
	let mut to_kick = HashSet::new();

	loop {
		// SLEEP maybe
	    poll.poll(&mut events, Some(tick_millis)).unwrap();

	    // handle ACCEPTING
	    for _event in events.iter() {
	    	match listener.accept() {
	    		Err(ref e) if e.kind() == ErrorKind::WouldBlock => (), //spurious wakeup
	    		Ok((client_stream, _peer_addr)) => {
    				println!("Pushing new client. managing {} clients now", client_data.len());
	    			client_data.push(
	    				ClientData {
	    					mm: Threadless::new(client_stream),
	    					state: ClientState::ExpectingHello,
	    				}
	    			);
	    		},
	    		Err(e) => {
	    			println!("[echo] listener socket died!");
	    			return;
	    		}, //socket has died
	    	}
	    }

	    // handle incoming messages
	    for (i, datum) in client_data.iter_mut().enumerate() {
	    	println!("visiting client at data index {}", i);
	    	loop {
	    		match datum.mm.try_recv::<Serverward>() {
	    			Ok(Serverward::Hello{name: name}) => {
	    				println!("got msg Serverward::Hello<name: {:?}> from client {:?}", &name, i);
	    				match datum.state {
							ClientState::ExpectingHello => {
								if game_state.name_taken(&name) {
									datum.mm.send(& Clientward::BadName);
									to_kick.insert(i);
								} else {
									let id = game_state.add_player(5, name.clone());
									println!("Client {:?} is playing in slot {:?}", i, id);
									datum.state = ClientState::Playing(id);
									datum.mm.send(
										& Clientward::Welcome { position: id }
									);
									datum.mm.send(
										& Clientward::GameState { state: game_state.clone() }
									);
									to_broadcast.push(
										Clientward::AddPlayer { start_h: 5, name: name },
									);
								}
							},
							ClientState::Playing(_slot) => {
								to_kick.insert(i); // got HELLO when not expecting it!
							},
	    				}
	    			}
	    			Ok(Serverward::MoveMe{ left: left}) => {
						match datum.state {
							ClientState::ExpectingHello => {
								println!("Kicking because got MoveMe when expected HELLO");
								to_kick.insert(i); // you need to hello first, dude
							},
							ClientState::Playing(slot) => {
								if game_state.move_piece(slot, left).is_ok() {
									println!("legal move!");
									to_broadcast.push(
										Clientward::MovePiece { pos: slot, left: left },
									);
								} else {
									println!("Illegal move!");
									to_kick.insert(i);
								}
							},
	    				}
	    			},
	    			Err(middleman::TryRecvError::ReadNotReady) => break,
	    			Err(e) => {
	    				println!("Oh no! client {:?} has problemz", i);
						to_kick.insert(i);
						break;
	    			},
	    		}
	    	}
	    }

	    // kick players
	    println!("kicking players");
	    if to_kick.len() > 0 {
	    	let mut indices = to_kick.drain().collect::<Vec<_>>();
	    	indices.sort();
	    	indices.reverse();
	    	for ii in indices {
	    		if let ClientState::Playing(slot) = client_data[ii].state {
	    			if game_state.remove_player(ii).is_ok() {
	    				to_broadcast.push(
	    					Clientward::RemovePlayer { pos: slot }
	    				);
	    			}
	    		}
	    		println!("Kicking client {:?}", ii);
	    		client_data.remove(ii);
	    	}
	    }

	    // broadcast
	    for msg in to_broadcast.drain(..) {
	    	for datum in client_data.iter_mut() {
	    		datum.mm.send(& msg);
	    	} 
	    }
	}
}


#[derive(Debug, Serialize, Deserialize)]
enum Serverward {
	Hello { name: String },
	MoveMe { left: bool },
}
impl Message for Serverward {}

#[derive(Debug, Serialize, Deserialize)]
enum Clientward {
	Welcome { position: usize },
	BadName,
	MovePiece { pos: usize, left: bool },
	AddPlayer { start_h: u32, name: String },
	RemovePlayer { pos: usize },
	GameState { state: GameState }, 
}
impl Message for Clientward {}

// fn server_handle(mut stream: TcpStream) {
// 	stream.set_nodelay(true).ok();
// 	let mut mm = Threadless::new(stream);
// 	if let Ok(Serverward::Hello{ name: name}) = mm.recv::<Serverward>() {
// 		println!("Got hello from {:?}", &name);
// 		mm.send(& Clientward::).ok();
// 	}
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
struct GameState {
	position: Vec<(u32, String)>,
}


const GAME_WIDTH: u32 = 16;
impl GameState {

	fn new() -> Self {
		GameState {
			position: vec![],
		}
	}

	fn name_taken(&self, name: &str) -> bool {
		for p in self.position.iter() {
			if p.1 == name {
				return true;
			}
		}
		false
	}

	fn pos_exists(&self, pos: usize) -> bool {
		self.position.len() > pos
	}
	fn move_piece(&mut self, pos: usize, left: bool) -> Result<(), &'static str> {
		if !self.pos_exists(pos) {
			return Err("No such position!")
		}
		let pos = &mut self.position[pos];
		if left {
			if pos.0 == 0 {
				return Err("too far left!")	
			}		
			pos.0 -= 1;	
			Ok(())
		} else { //right
			if pos.0 == GAME_WIDTH-1 {
				return Err("too far right!")	
			}
			pos.0 += 1;
			Ok(())
		}
	}

	fn add_player(&mut self, start_h: u32, name: String) -> usize {
		self.position.push((start_h, name));
		self.position.len() - 1
	}

	fn remove_player(&mut self, pos: usize) -> Result<(), &'static str>{
		if !self.pos_exists(pos) {
			return Err("Cant remove");
		}
		self.position.remove(pos);
		Ok(())
	}

	fn draw(&self) {
		println!();
		for (i, p) in self.position.iter().enumerate() {
			print!("{:?} [", p);
			for _ in 0..p.0 {
				print!(" ");
			}
			print!("$");
			for _ in p.0..GAME_WIDTH {
				print!(" ");
			}
			print!("] {:?}", &p.1);
			println!();
		}
	}
}


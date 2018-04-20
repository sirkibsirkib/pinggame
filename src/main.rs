
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
	Threadless,
};

use std::{
	thread,
	io::{
		ErrorKind,
		stdout,
		Write,
	},
	collections::HashMap,
	net::SocketAddr,
};
use mio::{
	Poll,
	Ready,
	PollOpt,
	Events,
	Token,
	Evented,
	tcp::{
		TcpListener,
		TcpStream,
	},
};
use mio_extras::channel::channel;

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

	let addr: SocketAddr = matches.value_of("ip").expect("NO IP??").parse().unwrap();
	println!("ADDR {:?}", &addr);



    match matches.value_of("name") {
    	Some(name) => client(&addr, name.to_owned()),
    	None => server(&addr),
    };
}

fn client(addr: &SocketAddr, name: String) {
	println!("CLIENT START with name `{}`", &name);

	let sock = TcpStream::connect(&addr).unwrap();
	let mut mm = Threadless::new(sock);
	mm.send(& Serverward::Hello{ name: name }).expect("hello failed");
	match mm.recv::<Clientward>() {
		Ok(Clientward::Welcome{ row: r}) => {
			client_play(r, mm)
		},
		x => {
			println!("CLIENT EXPECTED WELCOME {:?}", &x);
			return;
		},
	}
	
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum Direction { //TODO
	Left,
	Right,
}

fn client_play<M: Middleman>(row: usize, mut mm: M) {
	println!("I have been given row {:?}", row);
	let mut game_state = GameState::new();


    // handle ACCEPTING

	let (s, r) = channel(); //channel between keylistener thread and main
	let _handle = thread::spawn(move || {
		let mut buffer = String::new();
		println!("keylister going at it...");
		loop {
			println!("keylister reading line...");
			buffer.clear();
	    	std::io::stdin().read_line(&mut buffer).unwrap();
	    	match buffer.trim() {
	    		"l" => {
	    			s.send(Direction::Left).ok();
	    		},
	    		"r" => {
	    			s.send(Direction::Right).ok();
	    		},
	    		_ => {println!("read `{}`", &buffer);},
	    	}
		}
	});



	let poll = Poll::new().unwrap();
	poll.register(&r, Token(0), Ready::readable(),
				PollOpt::edge()).unwrap();
	let mut events = Events::with_capacity(128);
	let tick_millis = std::time::Duration::from_millis(300);

	loop {
   		poll.poll(&mut events, Some(tick_millis)).unwrap();
    	for _event in events.iter() {
    		if let Ok(d) = r.try_recv() {
				//request a move
				mm.send(& Serverward::MoveMe { dir: d } ).ok();
				println!("Sending req done");
			}
    	}

		match mm.try_recv::<Clientward>() {
			Ok(msg) => {
				;
				println!("Got {:?} from server...", msg);
				match msg {
					Clientward::Welcome { row: _r } => (), // wtf
					Clientward::MovePiece { row: r, dir: d } => {
						println!("moving piece in row {:?} in {:?}", r, d );
						game_state.move_piece(r, d).expect("move failed");
						game_state.draw();
					},
					Clientward::AddPlayer { start_h: s, name: n } => {
						game_state.add_player(s, n);
						game_state.draw();
					},
					Clientward::RemovePlayer { row: r } => {
						game_state.remove_player(r).expect("couldnt remove");
						game_state.draw();
					},
					Clientward::GameState { state: g } => {
						game_state = g;
						game_state.draw();
					}, 
					Clientward::KickingYou { kick_reason: kick_msg } => {
						println!("Ive been kicked! {:?}", kick_msg);
						return;
					} 
				}
			},
			Err(middleman::TryRecvError::ReadNotReady) => (),
			Err(e) => {
				println!("Something went wrong {:?}", e);
				return;
			}
		}
	}
}


enum ClientState {
	ExpectingHello,
	Playing(usize),
}
struct ClientData<M: Middleman> {
	mm: M,
	state: ClientState,
}


#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
struct ClientId(u32);


fn next_free_cid<M: Middleman>(client_data: &HashMap<ClientId, ClientData<M>>) -> ClientId {
	for id in 0.. {
		let cid = ClientId(id);
		if !client_data.contains_key(& cid) {
			return cid
		}
	}
	panic!("Ran out of IDS?");
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
	let mut client_data = HashMap::new();
	let mut to_broadcast = vec![];
	let mut send_to_all_but: Vec<(ClientId, Clientward)> = vec![];
	let mut to_kick: HashMap<ClientId, KickReason> = HashMap::new();

	loop {
		// SLEEP maybe
	    poll.poll(&mut events, Some(tick_millis)).unwrap();

	    // handle ACCEPTING
	    for _event in events.iter() {
	    	match listener.accept() {
	    		Err(ref e) if e.kind() == ErrorKind::WouldBlock => (), //spurious wakeup
	    		Ok((client_stream, _peer_addr)) => {
	    			let cid = next_free_cid(& client_data);
    				println!("Adding new client {:?}. managing {:?} clients now", cid, client_data.len());
	    			client_data.insert(
	    				cid,
	    				ClientData {
	    					mm: Threadless::new(client_stream),
	    					state: ClientState::ExpectingHello,
	    				},
	    			);
	    		},
	    		Err(e) => {
	    			println!("[echo] listener socket died! Err {:?}", e);
	    			return;
	    		}, //socket has died
	    	}
	    }

	    // handle incoming messages
	    for (&cid, datum) in client_data.iter_mut() {
	    	loop {
	    		match datum.mm.try_recv::<Serverward>() {
	    			Ok(Serverward::Hello{name: n}) => {
	    				println!("got msg Serverward::Hello<name: {:?}> from client {:?}", &n, cid);
	    				match datum.state {
							ClientState::ExpectingHello => {
								if game_state.name_taken(&n) {
									to_kick.insert(cid, KickReason::NameTaken);
								} else {
									let row = game_state.add_player(5, n.clone());
									println!("Client {:?} is playing in row {:?}", cid, row);
									datum.state = ClientState::Playing(row);
									let mut success = true;
									success &= datum.mm.send(
										& Clientward::Welcome { row: row }
									).is_ok();
									success &= datum.mm.send(
										& Clientward::GameState { state: game_state.clone() }
									).is_ok();
									if success {
										send_to_all_but.push(
											(
												cid,
												Clientward::AddPlayer { start_h: 5, name: n },
											)
										);
									} else {
										to_kick.insert(cid, KickReason::SocketErr);
									}
								}
							},
							ClientState::Playing(_slot) => {
								to_kick.insert(cid, KickReason::UnexpectedHello);
							},
	    				}
	    			}
	    			Ok(Serverward::MoveMe{ dir: d }) => {
						match datum.state {
							ClientState::ExpectingHello => {
								println!("Kicking because got MoveMe when expected HELLO");
								to_kick.insert(cid, KickReason::ExpectedHello);
							},
							ClientState::Playing(row) => {
								if game_state.move_piece(row, d).is_ok() {
									println!("legal move!");
									to_broadcast.push(
										Clientward::MovePiece { row: row, dir: d },
									);
								} else {
									println!("Illegal move!");
									to_kick.insert(cid, KickReason::IllegalMove);
								}
							},
	    				}
	    			},
	    			Err(middleman::TryRecvError::ReadNotReady) => break,
	    			Err(e) => {
	    				println!("Oh no! client {:?} has problemz {:?}", cid, e);
						to_kick.insert(cid, KickReason::SocketErr);
						break;
	    			},
	    		}
	    	}
	    }

	    // kick players
	    if to_kick.len() > 0 {
	    	for (cid, kick_msg) in to_kick.drain() {
	    		println!("Kicking {:?} for reason: {:?}", cid, & kick_msg);
	    		{
	    			let mut datum = client_data.get_mut(&cid).expect("kicking nonexistant?");
		    		if let ClientState::Playing(r) = datum.state {
		    			if game_state.remove_player(r).is_ok() {
		    				send_to_all_but.push((cid, Clientward::RemovePlayer { row: r }));
		    			}
		    		}
		    		let _ = datum.mm.send(& Clientward::KickingYou { kick_reason: kick_msg });
		    	}
	    		client_data.remove(&cid);
	    	}
	    }

	    // broadcast
	    for msg in to_broadcast.drain(..) {
	    	println!("Sending {:?} to all.", &msg);
	    	for (_cid, datum) in client_data.iter_mut() {
	    		let _ = datum.mm.send(& msg); //errors will be detected later
	    	} 
	    }

	    // send to all but cid
	    for (cid, msg) in send_to_all_but.drain(..) {
	    	println!("Sending {:?} to all but {:?}", &msg, cid);
	    	for (&cid2, mut datum) in client_data.iter_mut() {
	    		if cid != cid2 {
	    			let _ = datum.mm.send(& msg); //errors will be detected later
	    		}
	    	} 
	    }
	}
}


#[derive(Debug, Serialize, Deserialize)]
enum Serverward {
	Hello { name: String },
	MoveMe { dir: Direction },
}
impl Message for Serverward {}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum KickReason {
	ExpectedHello,
	UnexpectedHello,
	IllegalMove,
	NameTaken,
	SocketErr,
}

#[derive(Debug, Serialize, Deserialize)]
enum Clientward {
	Welcome { row: usize },
	MovePiece { row: usize, dir: Direction },
	AddPlayer { start_h: u32, name: String },
	RemovePlayer { row: usize },
	GameState { state: GameState }, 
	KickingYou{ kick_reason: KickReason },
}
impl Message for Clientward {}


#[derive(Deserialize, Serialize, Debug, Clone)]
struct GameState {
	rows: Vec<(u32, String)>,
}


const GAME_WIDTH: u32 = 16;
impl GameState {

	fn new() -> Self {
		GameState {
			rows: vec![],
		}
	}

	fn name_taken(&self, name: &str) -> bool {
		for p in self.rows.iter() {
			if p.1 == name {
				return true;
			}
		}
		false
	}

	fn row_exists(&self, row: usize) -> bool {
		self.rows.len() > row
	}
	fn move_piece(&mut self, row: usize, dir: Direction) -> Result<(), &'static str> {
		if !self.row_exists(row) {
			return Err("No such row!")
		}
		let row_h = &mut self.rows[row];
		match dir {
			Direction::Left => {
				if row_h.0 == 0 {
					return Err("too far left!")	
				}		
				row_h.0 -= 1;	
				Ok(())
			},
			Direction::Right => {
				if row_h.0 == GAME_WIDTH-1 {
					return Err("too far right!")	
				}
				row_h.0 += 1;
				Ok(())
			},
		}
	}

	fn add_player(&mut self, start_h: u32, name: String) -> usize {
		self.rows.push((start_h, name));
		self.rows.len() - 1
	}

	fn remove_player(&mut self, row: usize) -> Result<(), &'static str>{
		if !self.row_exists(row) {
			return Err("Cant remove");
		}
		self.rows.remove(row);
		Ok(())
	}

	fn draw(&self) {
		println!();
		for (i, row_h) in self.rows.iter().enumerate() {
			print!("{:?} [", i);
			for _ in 0..row_h.0 {
				print!(" ");
			}
			print!("$");
			for _ in row_h.0..GAME_WIDTH {
				print!(" ");
			}
			print!("] {:?}", &row_h.1);
			println!();
		}
		stdout().flush();
	}
}


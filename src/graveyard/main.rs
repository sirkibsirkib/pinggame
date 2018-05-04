
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
	thread,
	io::{
		ErrorKind,
		stdout,
		Write,
	},
	net,
	collections::HashMap,
	net::SocketAddr,
};
use mio::{
	Poll,
	Ready,
	PollOpt,
	Events,
	Token,
	tcp,
};
use mio_extras::channel::{
	channel,
	Sender,
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

	let addr: SocketAddr = matches.value_of("ip").expect("NO IP??").parse().unwrap();
	println!("ADDR {:?}", &addr);



    match matches.value_of("name") {
    	Some(name) => client(&addr, name.to_owned()),
    	None => server(&addr),
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
enum Direction { //TODO
	Left,
	Right,
}


fn handle_clientward(msg: Clientward, game_state: &mut GameState) -> bool {
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
			*game_state = g;
			game_state.draw();
		}, 
		Clientward::KickingYou { kick_reason: kick_msg } => {
			println!("Ive been kicked! {:?}", kick_msg);
			return true;
		} 
	};
	false
}


fn key_loop(s: Sender<Direction>) {
	let mut buffer = String::new();
	println!("keylister going at it...");
	loop {
		// println!("keylister reading line...");
		buffer.clear();
    	std::io::stdin().read_line(&mut buffer).unwrap();
    	println!("keylister sending {:?}", &buffer);
    	match buffer.trim() {
    		"l" => {
    			s.send(Direction::Left).ok();
    		},
    		"r" => {
    			s.send(Direction::Right).ok();
    		},
    		s => {println!("read `{}`", s);},
    	}
	}
}
const SERVER_TOK: Token = Token(0);
const KEY_TOK: Token = Token(1);


fn client(addr: &SocketAddr, name: String) {
	let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(256);
	let mut game_state = GameState::new();

	let sock = TcpStream::connect(&addr).unwrap();
	let mut mm = Middleman::new(sock);
    poll.register(&mm, SERVER_TOK, Ready::readable(),
                  PollOpt::edge()).unwrap();
    let mut spillover = vec![];

    // keylistener thread
	let (s, r) = channel(); 
    poll.register(&r, KEY_TOK, Ready::readable(), PollOpt::edge()).unwrap();
    let _handle = thread::spawn(move || {
		key_loop(s);
	});

	mm.send(& Serverward::Hello{ name: name }).expect("hello failed");
	let row = match mm.recv_blocking(&poll, &mut events, SERVER_TOK, &mut spillover, None) {
		Ok(Some(Clientward::Welcome {row : r})) => r,
		x => {
			println!("unexpected welcome {:?}", &x);
			panic!("FAK");
		},
	};

	println!("I have been given row {:?}", row);
	let mut game_going = true;

	while game_going {
   		poll.poll(&mut events, None).unwrap();
		// println!("events...");
    	for event in events.iter().chain(spillover.drain(..)) {
    		// println!("token {:?}", event.token());
    		match event.token() {
    			SERVER_TOK => {
    				let (_, res) = mm.try_recv_all_map::<_, Clientward>(|mm, msg| {
			    		if handle_clientward(msg, &mut game_state) {
			    			game_going = false;
			    		}
			    	}).1.expect("something went wrong");
    				if let Err(err) = res {
    					println!("server connection issue! {:?}", err);
    					game_going = false;
    				}
    			},
    			KEY_TOK => {
    				// println!("key event!!");
    				if let Ok(d) = r.try_recv() {
    					mm.send(& Serverward::MoveMe { dir: d } ).ok();
    					mm.write_out().ok();
						println!("Sending req done");
    				}
    			}
    			_ => unreachable!(),
    		}
    	}
	}
	println!("game over!");
}


enum ClientState {
	ExpectingHello,
	Playing(usize),
}
struct ClientData {
	mm: Middleman,
	state: ClientState,
}


// #[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
// struct ClientId(u32);
type ClientId = Token;


fn next_free_cid(client_data: &ClientDataMap) -> ClientId {
	for id in 1.. {
		let cid = Token(id);
		if !client_data.contains_key(& cid) {
			return cid
		}
	}
	panic!("Ran out of TOKENS?");
}


fn handle_new_socket(poll: &Poll, listener: &TcpListener, client_data: &mut ClientDataMap) {
	match listener.accept() {
		// Err(ref e) if e.kind() == ErrorKind::WouldBlock => (), //spurious wakeup
		Ok((client_stream, _peer_addr)) => {
			let cid = next_free_cid(client_data as &ClientDataMap);
			println!("Adding new client {:?}. managing {:?} clients now", cid, client_data.len()+1);
			let std_stream = ::std::net::TcpStream::from_raw_fd(

			);
			let mm = Middleman::new(client_stream);
    		poll.register(&mm, cid, Ready::readable(),
                  PollOpt::edge()).unwrap();
			client_data.insert(
				cid,
				ClientData {
					mm:		mm,
					state:	ClientState::ExpectingHello,
				},
			);
		},
		Err(e) => {
			println!("[echo] listener socket died! Err {:?}", e);
			return;
		},
	}
}



fn handle_incoming(msg: Serverward, cid: Token, datum: &mut ClientData, server_state: &mut ServerState) {
	println!("Server got msg {:?}", &msg);
	match msg {
		Serverward::MoveMe{ dir: d } => {
			match datum.state {
				ClientState::ExpectingHello => {
					println!("Kicking because got MoveMe when expected HELLO");
					server_state.to_kick.insert(cid, KickReason::ExpectedHello);
				},
				ClientState::Playing(row) => {
					if server_state.game_state.move_piece(row, d).is_ok() {
						println!("legal move!");
						server_state.to_broadcast.push(
							Clientward::MovePiece { row: row, dir: d },
						);
					} else {
						println!("Illegal move!");
						server_state.to_kick.insert(cid, KickReason::IllegalMove);
					}
				},
			}
		},
		Serverward::Hello{name: n} => {
			println!("got msg Serverward::Hello<name: {:?}> from client {:?}", &n, cid);
			match datum.state {
				ClientState::ExpectingHello => {
					if server_state.game_state.name_taken(&n) {
						server_state.to_kick.insert(cid, KickReason::NameTaken);
					} else {
						let row = server_state.game_state.add_player(5, n.clone());
						println!("Client {:?} is playing in row {:?}", cid, row);
						datum.state = ClientState::Playing(row);
						let mut success = true;
						success &= datum.mm.send(
							& Clientward::Welcome { row: row }
						).is_ok();
						success &= datum.mm.send(
							& Clientward::GameState { state: server_state.game_state.clone() }
						).is_ok();
						if success {
							server_state.send_to_all_but.push(
								(
									cid,
									Clientward::AddPlayer { start_h: 5, name: n },
								)
							);
						} else {
							server_state.to_kick.insert(cid, KickReason::SocketErr);
						}
					}
				},
				ClientState::Playing(_slot) => {
					server_state.to_kick.insert(cid, KickReason::UnexpectedHello);
				},
			}
		},
	}	
}

type ToKick = HashMap<ClientId, KickReason>;
type SendToAllBut = Vec<(ClientId, Clientward)>;
type ClientDataMap = HashMap<Token, ClientData>;
type ToBroadcast = Vec<Clientward>;

const LISTENER_TOK: Token = Token(0);

struct ServerState {
	to_kick: ToKick,
	to_broadcast: ToBroadcast,
	send_to_all_but: SendToAllBut,
	game_state: GameState,
}

fn server(addr: &SocketAddr) {
	let listener = TcpListener::bind(addr).unwrap();
	let poll = Poll::new().unwrap();
	poll.register(&listener, LISTENER_TOK, Ready::readable(),
		PollOpt::edge()).unwrap();

	let mut events = Events::with_capacity(256);

	let mut server_state = ServerState {
		to_kick: HashMap::new(),
		send_to_all_but: vec![],
		to_broadcast: vec![],
		game_state: GameState::new(),
	};
	let mut client_data: ClientDataMap = HashMap::new();

	loop {
		// poll for new activity
	    poll.poll(&mut events, None).unwrap();

	    // handle events
	    // println!("events");
	    for event in events.iter() {
	    	let token = event.token();
	    	// println!("token {:?}", token);
	    	if token == LISTENER_TOK {
    			handle_new_socket(&poll, &listener, &mut client_data);
	    	} else if let Some(mut datum) = client_data.get_mut(& token) {
	    		let (_, res) = datum.mm.try_recv_all_map(|mm, msg| {
	    			handle_incoming(msg, cid, &mut datum, &mut server_state);
	    		});
	    		if let Err(err) = res {
					println!("Oh no! client {:?} has problemz {:?}", token, err);
					server_state.to_kick.insert(token, KickReason::SocketErr);
					break;
	    		}
    		} else {
    			println!("Unknown token {:?}", token);
    			panic!("Unknown Token");
    		}
	    }

	    // kick players
    	for (cid, kick_msg) in server_state.to_kick.drain() {
    		println!("Kicking {:?} for reason: {:?}", cid, & kick_msg);
    		{
    			let mut datum = client_data.get_mut(&cid).expect("kicking nonexistant?");
    			poll.deregister(&datum.mm).expect("dereg failed");
	    		if let ClientState::Playing(r) = datum.state {
	    			if server_state.game_state.remove_player(r).is_ok() {
	    				server_state.send_to_all_but.push((cid, Clientward::RemovePlayer { row: r }));
	    			}
	    		}
	    		let _ = datum.mm.send(& Clientward::KickingYou { kick_reason: kick_msg });
	    	}
    		client_data.remove(&cid);
    	}

	    // broadcast
	    for msg in server_state.to_broadcast.drain(..) {
	    	println!("Sending {:?} to all.", &msg);
	    	for (_cid, datum) in client_data.iter_mut() {
	    		if datum.mm.send(& msg).is_ok() {
	    			datum.mm.write_out().ok();
	    		}
	    	} 
	    }

	    // send to all but cid
	    for (cid, msg) in server_state.send_to_all_but.drain(..) {
	    	println!("Sending {:?} to all but {:?}", &msg, cid);
	    	for (&cid2, mut datum) in client_data.iter_mut() {
	    		if cid != cid2 {
	    			println!("...sending to {:?}", cid2);
	    			if datum.mm.send(& msg).is_ok() {
	    				datum.mm.write_out().ok();
	    			}
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
		let _ = stdout().flush();
	}
}


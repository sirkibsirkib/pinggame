
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
    	Some(name) => client(&addr, name.to_owned()).ok(),
    	None => server(&addr).ok(),
    };

    // match matches.occurrences_of("server") {
    //     0 => client(&addr).ok(),
    //     _ => server(&addr).ok(),
    // };
}

fn client(addr: &SocketAddr, name: String) -> Result<(), std::io::Error> {
	println!("CLIENT START");
	// let client_tok = Token(0);
	// let poll = Poll::new().unwrap();
	let sock = TcpStream::connect(&addr)?;
	let mut mm = Threadless::new(sock);
	mm.send(& Serverward::Hello{ name: name });
	match mm.recv::<Clientward>() {
		Ok(Clientward::Welcome{ position: p}) => {
			client_play(p, mm)
		},
		x => {
			println!("CLIENT EXPECTED WELCOME {:?}", &x);
			Err("")
		},
	}
	
}

fn client_play<M: Middleman>(pos: usize, mm: M) -> Result<(), std::io::Error> {
	let mut game_state = GameState::new();
	//TODO
	Ok(())
}


// spawn an echo server on localhost. return the thread handle and the bound ip addr.
fn server(addr: &SocketAddr) -> Result<(), std::io::Error> {
	let listener = TcpListener::bind(addr)?;
	let poll = Poll::new().unwrap();
	poll.register(&listener, Token(0), Ready::readable(),
		PollOpt::edge()).unwrap();
	let mut events = Events::with_capacity(128);

	let mut game_state = GameState::new();
	// let names => 

	loop {
	    poll.poll(&mut events, None).unwrap();
	    for _event in events.iter() {
	    	match listener.accept() {
	    		Err(ref e) if e.kind() == ErrorKind::WouldBlock => (), //spurious wakeup
	    		Ok((client_stream, peer_addr)) => {
	        		println!("server connects");
	    			thread::Builder::new()
		        	.name(format!("handler_for_client@{:?}", peer_addr))
		        	.spawn(move || {
		        		server_handle(client_stream);
		        	}).unwrap();
	    		},
	    		Err(e) => {
	    			println!("[echo] listener socket died!");
	    			return Err(e);
	    		}, //socket has died
	    	}
	    }
	}
}

#[derive(Debug, Serialize, Deserialize)]
enum Serverward {
	Hello { name: String },
	BadName,
	MoveLeft,
	MoveRight,
}
impl Message for Serverward {}

#[derive(Debug, Serialize, Deserialize)]
enum Clientward {
	Welcome { position: usize },
	AddPlayer { start_h: u32, name: String },
	RemovePlayer { pos: usize },
}
impl Message for Clientward {}

fn server_handle(mut stream: TcpStream) {
	stream.set_nodelay(true).ok();
	let mut mm = Threadless::new(stream);
	loop {
		match mm.recv::<Serverward>() {
			Ok(msg) => {},
			Err(e) => {
				println!("Client disconnected");
				return;
			},
		}
	}
}


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

	fn pos_exists(&self, pos: usize) -> bool {
		self.position.len() > pos
	}
	fn move_piece(&mut self, pos: usize, left: bool) -> Result<(), &'static str> {
		if !self.pos_exists() {
			return Err("No such position!")
		}
		let pos = &mut self.position[pos];
		if left {
			if pos.0 == 0 {
				return Err("too far left!")	
			}		
			pos.0 -= 1;	
		} else { //right
			if pos.0 == GAME_WIDTH-1 {
				return Err("too far right!")	
			}
			pos.0 += 1;
		}
	}

	fn add_player(&mut self, start_h: u32, name: String) {
		self.position.push((start_h, name));
	}

	fn remove_player(&mut self, pos: usize) -> Result<(), &'static str>{
		match self.position.remove(pos) {
			Some(_) => Ok(()),
			None => Err("nothing to remove!"),
		}
	}

	fn draw(&self) {
		for (i, p) in self.position.iter().enumerate() {
			print!("{} [", p);
			for _ in 0..p.0 {
				print!(" ");
			}
			print!("$");
			for _ in p.0..GAME_WIDTH {
				print!(" ");
			}
			print!("] {}", &p.1);
		}
	}
}


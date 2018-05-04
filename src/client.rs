use ::common::*;
use ::game::*;

use std::{
	net::SocketAddr,
};

pub fn client_enter(addr: &SocketAddr, moniker: Moniker) {
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
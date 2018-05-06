
use ::game::*;
use ::middleman;
use ::std::{self,
	time::Duration,
};
use super::mio;

/////////////////////////////////////////////////////////////

pub const MOVE_PERIOD: Duration = Duration::from_millis(130);
pub const BOT_MOVE_PERIOD: Duration = Duration::from_millis(400);
pub const DIR_CHOICES: [Direction; 4] = [
	Direction::Up, Direction::Down, Direction::Left, Direction::Right
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Clientward {
	Welcome(GameStateEssence),
	AddPlayer(Moniker, Coord2D),
	RemovePlayer(Moniker),
	UpdMove(Moniker, Direction),
	ErrorTakenMoniker,
	ErrorIllegalMove,
	ErrorSocketDead,
	ErrorExpectedReq,
	ErrorExpectedHello,
}
impl middleman::Message for Clientward {}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Serverward {
	Hello(Moniker),
	ReqMove(Direction),
}
impl middleman::Message for Serverward {}

/////////////////////////////////////////////////////////////

pub type MioStream = mio::net::TcpStream;
pub type StdStream = std::net::TcpStream;
pub type MioListener = mio::net::TcpListener;
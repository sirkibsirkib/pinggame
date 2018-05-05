
use ::game::*;
use ::middleman;
use ::std;
use super::mio;

/////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Clientward {
	Welcome(GameState),
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
use rand::{
	thread_rng,
	Rng,
	SeedableRng,
	XorShiftRng,
};
use bitset::BitSet;
use std::{
	fmt,
	collections::{
		HashMap,
		HashSet,
	},
};



#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct LCGenerator {
	seed: u64,
}

impl LCGenerator { // Linear Congruential Generator
	const A: u64 = 1103515245;
	const C: u64 = 12345;
	const M: u64 = 0x100_000_000;

	pub fn new_random_seeded() -> Self {
		LCGenerator {
			seed: thread_rng().gen(),
		}
	}
}
impl Rng for LCGenerator {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.seed = (Self::A * self.seed + Self::C) % Self::M;
  		return self.seed as u32;
    }
    fn next_u64(&mut self) -> u64 {
        self.seed = (Self::A * self.seed + Self::C) % Self::M;
  		return self.seed;
    }
}



#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, Hash)]
pub struct Moniker(pub char);

pub type ValidMove = bool;
type GameStateSeed = [u32; 4];

fn new_random_seed() -> GameStateSeed {
	thread_rng().gen()
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerObject {
	pub coord: Coord2D,
	pub charge: u16,
}
impl PlayerObject {
	const POWER_LIMIT: u16 = 3;
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameStateEssence { //everything that CANNOT be generated
	players: HashMap<Moniker, PlayerObject>, 
	wall_default_seed: GameStateSeed,
	wall_override: HashMap<Coord2D, bool>,
	power_blobs: HashSet<Coord2D>,
	sync_rng: LCGenerator,
}
pub struct GameState { //all but `essence` can be generated from `essence`
	essence: GameStateEssence,
	wall_default: Vec<BitSet>,
	non_wall_spaces: usize,
}

impl fmt::Debug for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GameState with essence {:?}", &self.essence)
    }
}


impl GameState { // basic stuff
	pub const WIDTH: u16 = 30;
	pub const HEIGHT: u16 = 22;
	pub const TOTAL_COORDS: usize =
		Self::WIDTH as usize * Self::HEIGHT as usize;
	pub const NUM_POWER_BLOBS: u8 = 3;

	#[inline]
	pub fn get_essence(& self) -> &GameStateEssence {
		& self.essence
	}

	#[inline]
	pub fn num_players(&self) -> usize {
		self.essence.players.len()
	}

	#[inline]
	pub fn num_empty_coords(&self) -> usize {
		self.non_wall_spaces - self.num_players()
	}

	#[inline]
	pub fn contains_player(&self, moniker: Moniker) -> bool {
		self.essence.players.contains_key(&moniker)
	}

	fn get_player_at(&self, coord: Coord2D) -> Option<&PlayerObject> {
		for player in self.essence.players.values() {
			if player.coord == coord {
				return Some(player);
			}
		}
		None
	}

	// fn get_mut_player_at(&mut self, coord: Coord2D) -> Option<&mut PlayerObject> {
	// 	for player in self.essence.players.values_mut() {
	// 		if player.coord == coord {
	// 			return Some(player);
	// 		}
	// 	}
	// 	None
	// }

	#[inline]
	fn is_player_at(&self, coord: Coord2D) -> bool {
		self.get_player_at(coord).is_some()
	}

	#[inline]
	pub fn is_wall_at(&self, coord: Coord2D) -> bool {
		self.essence.wall_override.get(&coord)
		.map(|x| *x)
		.unwrap_or_else(|| {
			self.wall_default[coord.y as usize]
			.test(coord.x as usize)
		})
	}

	#[inline]
	pub fn is_blob_at(&self, coord: Coord2D) -> bool {
		self.essence.power_blobs.contains(&coord)
	}

	pub fn is_something_at(&self, coord: Coord2D) -> bool {
		self.is_player_at(coord)
		|| self.is_wall_at(coord)
		|| self.is_blob_at(coord)
	}

	fn set_wall_value(&mut self, coord: Coord2D, value: bool) {
		if value != self.is_wall_at(coord) {
			self.essence.wall_override.insert(coord, value);
		}
	}

	pub fn try_add_player(&mut self, moniker: Moniker, coord: Coord2D) -> ValidMove {
		if self.essence.players.contains_key(&moniker)
		|| self.is_something_at(coord) {
			return false
		}
		let obj = PlayerObject {
			coord: coord,
			charge: 3,
		};
		self.essence.players.insert(moniker, obj);
		true
	}

	pub fn try_remove_player(&mut self, moniker: Moniker) -> ValidMove {
		self.essence.players.remove(&moniker).is_some()
	}

	pub fn coord_on_boundary(coord: Coord2D) -> bool {
		coord.x == 0
		|| coord.y == 0
		|| coord.x == Self::WIDTH-1
		|| coord.y == Self::HEIGHT-1
	}

	pub fn coord_would_exit(coord: Coord2D, dir: Direction) -> bool {
		match dir {
			Direction::Up => coord.y == 0,
			Direction::Down => coord.y == Self::HEIGHT-1,
			Direction::Left => coord.x == 0,
			Direction::Right => coord.x == Self::WIDTH-1,
		}
	}

	pub fn player_iter(&self) -> PlayerIter {
		PlayerIter(self.essence.players.iter())
	}

	pub fn blob_iter(&self) -> BlobIter {
		BlobIter(self.essence.power_blobs.iter())
	}
	// WAITING FOR IMPL TRAIT
	pub fn coord_iter(&self) -> CoordIter {
		CoordIter { next: Coord2D::NULL }
	}

	pub fn empty_cell_ratio(&self) -> f32 {
		(self.num_empty_coords() as f32)
		/ (Self::TOTAL_COORDS as f32)
	}
}

impl GameState { /// major stuff

	pub fn new_random() -> Self {
		let essence = GameStateEssence {
			players: HashMap::new(), 
			wall_default_seed: new_random_seed(),
			wall_override: HashMap::new(),
			power_blobs: HashSet::new(),
			sync_rng: LCGenerator::new_random_seeded(),
		};
		let mut x = Self::from_essence(essence);
		for _ in 0..Self::NUM_POWER_BLOBS {
			let coord = x.random_free_spot()
				.expect("nowhere to put blob");
			x.essence.power_blobs.insert(coord);
		}
		x
	}

	pub fn from_essence(essence: GameStateEssence) -> Self {
		// build default wall object. 
		let mut rng: XorShiftRng = SeedableRng::from_seed(essence.wall_default_seed);
		let mut wall_default = vec![];
		let mut wall_count = 0;
		for y in 0..Self::HEIGHT {
			let mut row = BitSet::with_capacity(Self::WIDTH as usize);
			for x in 0..Self::WIDTH {
				if Self::coord_on_boundary(Coord2D::new(x, y))
				|| rng.gen_weighted_bool(3) {
					row.set(x as usize, true);
					wall_count += 1;
				}
			}
			wall_default.push(row);
		}
		GameState {
			essence: essence,
			wall_default: wall_default,
			non_wall_spaces: Self::TOTAL_COORDS as usize - wall_count,
		}
	}

	pub fn random_free_spot(&self) -> Option<Coord2D> {
		match self.empty_cell_ratio() {
			x if x > 0.96 => None, // I give up
			x if x > 0.6 => { //linear select
				let mut rng = thread_rng();
				let choice_index = rng.gen_range(0, self.num_empty_coords());
				self.coord_iter()
				.filter(|&coord| !self.is_something_at(coord))
				.nth(choice_index)
			},
			_ => { // trial and error
				let mut rng = thread_rng();
				let mut coord;
				loop {
					coord = Coord2D::new(
						rng.gen_range(0, Self::WIDTH),
						rng.gen_range(0, Self::HEIGHT),
					);
					if !self.is_something_at(coord) {
						return Some(coord);
					}
				}
			}
		}
	}

	pub fn sync_random_free_spot(&mut self) -> Option<Coord2D> {
		match self.empty_cell_ratio() {
			x if x > 0.96 => None, // I give up
			x if x > 0.6 => { //linear select
				let ne = self.num_empty_coords();
				let choice_index = {
					let mut rng = &mut self.essence.sync_rng;
					rng.gen_range(0, ne)
				};
				self.coord_iter()
				.filter(|&coord| !self.is_something_at(coord))
				.nth(choice_index)
			},
			_ => { // trial and error
				loop {
					let coord = {
						let mut rng = &mut self.essence.sync_rng;
						Coord2D::new(
							rng.gen_range(0, Self::WIDTH),
							rng.gen_range(0, Self::HEIGHT),
						)
					};
					if !self.is_something_at(coord) {
						return Some(coord);
					}
				}
			}
		}
	}

	fn try_move_wall(&mut self, src: Coord2D, dir: Direction) -> ValidMove {
		if Self::coord_would_exit(src, dir)
		|| !self.is_wall_at(src) {
			return false; // wall doesn't exist or is on boundary
		}
		let dest = src.move_with(dir);
		if self.is_something_at(dest) {
			return false; // something preventing move
		}
		self.set_wall_value(src, false);
		self.set_wall_value(dest, true);
		true
	}

	pub fn move_moniker_in_dir(&mut self, moniker: Moniker, dir: Direction) -> ValidMove {
		if !self.contains_player(moniker) { return false; } // no such player
		let src = self.essence.players.get_mut(&moniker).unwrap().coord;

		if Self::coord_would_exit(src, dir) { return false; } // on boundary
		let dest = src.move_with(dir);

		if self.is_player_at(dest) { return false; } //obstructed
		if self.is_wall_at(dest) {
			if self.essence.players.get(&moniker).unwrap().charge > 0
			&& self.try_move_wall(dest, dir) {
				//successfully moved wall
				let player = self.essence.players.get_mut(&moniker).unwrap();
				player.coord = dest;
				player.charge -= 1;
				true
			} else {
				//failed to move wall 
				false
			}
		} else {
			// spot was free
			self.essence.players.get_mut(&moniker)
			.unwrap().coord = dest;
			if self.is_blob_at(dest) {
				self.essence.power_blobs.remove(&dest);
				let new_blob_at = self.sync_random_free_spot()
					.expect("nowhere to put blob");
				self.essence.power_blobs.insert(new_blob_at);

				let player = self.essence.players.get_mut(&moniker).unwrap();
				if player.charge < PlayerObject::POWER_LIMIT {
					player.charge += 1;
				}
			}
			true
		}
	}
}

pub struct PlayerIter<'a>(::std::collections::hash_map::Iter<'a, Moniker, PlayerObject>);
impl<'a> Iterator for PlayerIter<'a> {
    type Item = (&'a Moniker, &'a PlayerObject);
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

pub struct BlobIter<'a>(::std::collections::hash_set::Iter<'a, Coord2D>);
impl<'a> Iterator for BlobIter<'a> {
    type Item = &'a Coord2D;
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

pub struct CoordIter {
	next: Coord2D,
}
impl Iterator for CoordIter {
    type Item = Coord2D;
fn next(&mut self) -> Option<Self::Item> {
		if self.next.x >= GameState::WIDTH {
			self.next.x = 0;
			self.next.y += 1;
			if self.next.y >= GameState::HEIGHT {
				return None;
			}
		}
		let was = self.next;
		self.next = Coord2D::new(was.x+1, was.y);
		Some(was)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
    	let x = GameState::TOTAL_COORDS;
    	(x, Some(x))
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct Coord2D {
	pub x: u16,
	pub y: u16,	
}
impl Coord2D {
	pub const NULL: Coord2D = Coord2D { x:0, y:0 }; 

	#[inline]
	pub fn new(x: u16, y: u16) -> Self {
		Coord2D { x:x, y:y }
	}

	// ASSUMES ITS VALID
	pub fn move_with(self, dir: Direction) -> Coord2D {
		match dir {
			Direction::Up => Coord2D::new(self.x, self.y-1),
			Direction::Down => Coord2D::new(self.x, self.y+1),
			Direction::Left => Coord2D::new(self.x-1, self.y),
			Direction::Right => Coord2D::new(self.x+1, self.y),
		}
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum Direction {
	Left, Right, Up, Down,
}

use rand::{
	thread_rng,
	Rng,
	SeedableRng,
	XorShiftRng,
};
use bitset::BitSet;
use std::{
	fmt,
	collections::HashMap,
};


#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, Hash)]
pub struct Moniker(pub char);

pub type ValidMove = bool;
type GameStateSeed = [u32; 4];

fn new_random_seed() -> GameStateSeed {
	thread_rng().gen()
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerObject {
	coord: Coord2D,
	charge: u16,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameStateEssence { //everything that CANNOT be generated
	players: HashMap<Moniker, PlayerObject>, 
	rand_seed: GameStateSeed,
	wall_override: HashMap<Coord2D, bool>,
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

	#[inline]
	pub fn get_essence(& self) -> &GameStateEssence {
		& self.essence
	}

	#[inline]
	pub fn num_monikers(&self) -> usize {
		self.essence.monikers.len()
	}

	#[inline]
	pub fn contains_moniker(&self, moniker: Moniker) -> bool {
		self.index_of_moniker(moniker).is_some()
	}

	#[inline]
	fn is_moniker_at(&self, coord: Coord2D) -> bool {
		self.index_of_moniker_by_coord(coord).is_some()
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

	pub fn is_something_at(&self, coord: Coord2D) -> bool {
		self.is_moniker_at(coord)
		|| self.is_wall_at(coord)
	}

	fn set_wall_value(&mut self, coord: Coord2D, value: bool) {
		if value != self.is_wall_at(coord) {
			self.essence.wall_override.insert(coord, value);
		}
	}

	pub fn add_player(&mut self, moniker: Moniker, obj: PlayerObject) -> ValidMove {
		if self.players.contains_key(&moniker)
		|| self.is_something_at(obj.coord) {
			return false
		}
		self.players.insert(moniker, obj);
		true
	}

	pub fn remove_player(&mut self, moniker: Moniker) -> ValidMove {
		self.players.remove(&moniker).is_some()
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
			Direction::Down => coord.y >= Self::HEIGHT-1,
			Direction::Left => coord.x == 0,
			Direction::Right => coord.x >= Self::WIDTH-1,
		}
	}

	pub fn moniker_iter(&self) -> MonikerIter {
		MonikerIter(self.essence.monikers.iter())
	}
	// WAITING FOR IMPL TRAIT
	pub fn coord_iter(&self) -> CoordIter {
		CoordIter { next: Coord2D::NULL }
	}
}

impl GameState { /// major stuff
	pub const WIDTH: u16 = 30;
	pub const HEIGHT: u16 = 22;
	pub const TOTAL_COORDS: usize =
		Self::WIDTH as usize * Self::HEIGHT as usize;

	pub fn new_random() -> Self {
		let essence = GameStateEssence {
			monikers: vec![], 
			rand_seed: new_random_seed(),
			wall_override: HashMap::new(),
		};
		Self::from_essence(essence)
	}

	pub fn from_essence(essence: GameStateEssence) -> Self {
		let mut rng: XorShiftRng = SeedableRng::from_seed(essence.rand_seed);
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

	pub fn empty_cell_ratio(&self) -> f32 {
		let spaces_left = self.non_wall_spaces - self.num_monikers();
		let total_spaces = Self::WIDTH*Self::HEIGHT;
		(spaces_left as f32) / (total_spaces as f32)
	}

	pub fn random_free_spot(&self) -> Option<Coord2D> {
		let spaces_left = self.non_wall_spaces - self.num_monikers();
		match self.empty_cell_ratio() {
			x if x > 0.96 => None,
			x if x > 0.7 => { //linear select
				let mut rng = thread_rng();
				let choice_index = rng.gen_range(0, spaces_left);
				self.coord_iter()
				.filter(|&coord| !self.is_wall_at(coord))
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
					if self.is_wall_at(coord) { continue; }
					if self.index_of_moniker_by_coord(coord).is_none() {
						return Some(coord);
					}
				}
			}
		}
	}

	fn index_of_moniker_by_coord(&self, coord: Coord2D) -> Option<usize> {
		for (i, &(c, _m)) in self.essence.monikers.iter().enumerate() {
			if c == coord { return Some(i) }
		}
		None
	}

	fn index_of_moniker(&self, moniker: Moniker) -> Option<usize> {
		for (i, &(_c, m)) in self.essence.monikers.iter().enumerate() {
			if m == moniker { return Some(i) }
		}
		None
	}

	fn try_move_wall(&mut self, src: Coord2D, dir: Direction) -> ValidMove {
		if Self::coord_would_exit(src, dir)
		|| !self.is_wall_at(src) {
			return false; // wall doesn't exist or is on boundary
		}
		let dest = src.move_with(dir);
		if self.is_wall_at(dest)
		|| self.is_moniker_at(dest) {
			return false; // something preventing move
		}
		self.set_wall_value(src, false);
		self.set_wall_value(dest, true);
		true
	}

	pub fn move_moniker_in_dir(&mut self, moniker: Moniker, dir: Direction) -> ValidMove {
		self.index_of_moniker(moniker)
		.and_then(|index| {
			let current_pos = self.essence.monikers[index].0;
			if Self::coord_would_exit(current_pos, dir) {
				None
			} else {
				let dest = current_pos.move_with(dir);
				if self.is_moniker_at(dest) {
					None // someone in the way
				} else if self.is_wall_at(dest) {
					// wall in the way
					if self.try_move_wall(dest, dir) {
						Some(()) // success
					} else {
						None
					}
				} else {
					// empty space at `dest`
					self.essence.monikers[index].0 = dest;
					Some(())
				}
			}
		}).is_some()
	}
}

pub struct MonikerIter<'a>(::std::slice::Iter<'a, (Coord2D, Moniker)>);
impl<'a> Iterator for MonikerIter<'a> {
    type Item = &'a (Coord2D, Moniker);
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

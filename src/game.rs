use rand::{
	thread_rng,
	Rng,
	SeedableRng,
	XorShiftRng,
};
use bitset::BitSet;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, Hash)]
pub struct Moniker(pub char);

pub type ValidMove = bool;
type GameStateSeed = [u32; 4];

fn new_random_seed() -> GameStateSeed {
	thread_rng().gen()
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameStateEssence {
	monikers: Vec<(Coord2D, Moniker)>, 
	rand_seed: GameStateSeed,
}
pub struct GameState {
	essence: GameStateEssence,
	walls: Vec<BitSet>,
}

impl fmt::Debug for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GameState with essence {:?}", &self.essence)
    }
}

impl GameState {
	pub const WIDTH: u16 = 30;
	pub const HEIGHT: u16 = 22;

	pub fn new_random() -> Self {
		let essence = GameStateEssence {
			monikers: vec![], 
			rand_seed: new_random_seed(),
		};
		Self::from_essence(essence)
	}

	pub fn from_essence(essence: GameStateEssence) -> Self {
		let mut rng: XorShiftRng = SeedableRng::from_seed(essence.rand_seed);
		let mut walls = vec![];
		for _y in 0..Self::HEIGHT {
			let mut row = BitSet::with_capacity(Self::WIDTH as usize);
			for x in 0..Self::WIDTH {
				if rng.gen_weighted_bool(3) {
					row.set(x as usize, true);
				}
			}
			walls.push(row);
		}
		GameState {
			essence: essence,
			walls: walls,
		}
	}

	pub fn get_essence(& self) -> &GameStateEssence {
		& self.essence
	}

	pub fn contains_moniker(&self, moniker: Moniker) -> bool {
		self.index_of_moniker(moniker).is_some()
	}

	pub fn random_free_spot(&self) -> Option<Coord2D> {
		if self.essence.monikers.len()/2 >= (Self::WIDTH * Self::HEIGHT) as usize {
			return None
			// TODO linear probing or something better. Fine for now
		}
		let mut rng = thread_rng();
		let mut coord;
		loop {
			coord = Coord2D::new(
				rng.gen_range(0, Self::WIDTH),
				rng.gen_range(0, Self::HEIGHT),
			);
			if self.index_of_moniker_by_coord(coord).is_none() {
				return Some(coord);
			}
		}
	}

	pub fn try_remove_moniker(&mut self, moniker: Moniker) -> ValidMove {
		if let Some(index) = self.index_of_moniker(moniker) {
			self.essence.monikers.remove(index);
			true
		} else { false }
	}

	pub fn try_put_moniker(&mut self, moniker: Moniker, coord: Coord2D) -> ValidMove {
		if self.index_of_moniker_by_coord(coord).is_none() {
			self.essence.monikers.push((coord, moniker));
			true
		} else { false }
		
	}

	fn index_of_moniker_by_coord(&self, coord: Coord2D) -> Option<usize> {
		for (i, &(c, _m)) in self.essence.monikers.iter().enumerate() {
			if c == coord {
				return Some(i)
			}
		}
		None
	}

	fn index_of_moniker(&self, moniker: Moniker) -> Option<usize> {
		for (i, &(_c, m)) in self.essence.monikers.iter().enumerate() {
			if m == moniker {
				return Some(i)
			}
		}
		None
	}

	pub fn move_moniker_in_dir(&mut self, moniker: Moniker, dir: Direction) -> ValidMove {
		let current_pos;
		if let Some(index) = self.index_of_moniker(moniker) {
			let &mut (c, _p) = &mut self.essence.monikers[index];
			current_pos = c;
		} else {
			return false // no such moniker
		};
		if self.can_move_at(current_pos, dir) {
			let index = self.index_of_moniker(moniker).unwrap();
			let &mut (ref mut c, _p) = &mut self.essence.monikers[index];
			c.move_with(dir);
			true
		} else {
			false
		}
	} 

	pub fn can_move_at(&self, coord: Coord2D, dir: Direction) -> bool {
		if !match dir {
			Direction::Up => coord.y > 0,
			Direction::Down => coord.y < Self::HEIGHT-1,
			Direction::Left => coord.x > 0,
			Direction::Right => coord.x < Self::WIDTH-1,
		} {
			return false; // end of boundary
		}
		let coord2 = match dir {
			Direction::Up => Coord2D { x:coord.x, y:coord.y-1 },
			Direction::Down => Coord2D { x:coord.x, y:coord.y+1 },
			Direction::Left => Coord2D { x:coord.x-1, y:coord.y },
			Direction::Right => Coord2D { x:coord.x+1, y:coord.y },
		};
		return !self.is_wall_at(coord2)
		&& self.index_of_moniker_by_coord(coord2).is_none() // no moniker there
	}

	#[inline]
	fn is_wall_at(&self, coord: Coord2D) -> bool {
		self.walls[coord.y as usize]
		.test(coord.x as usize)
	}

	pub fn moniker_iter(&self) -> MonikerIter {
		MonikerIter(self.essence.monikers.iter())
	}
	pub fn wall_iter(&self) -> WallIter {
		WallIter {
			bit_grid: &self.walls,
			next: Coord2D::new(0, 0),
		}
	}
}

pub struct MonikerIter<'a>(::std::slice::Iter<'a, (Coord2D, Moniker)>);
impl<'a> Iterator for MonikerIter<'a> {
    type Item = &'a (Coord2D, Moniker);
    fn next(&mut self) -> Option<Self::Item> { self.0.next() }
    fn size_hint(&self) -> (usize, Option<usize>) { self.0.size_hint() }
}

pub struct WallIter<'a> {
	bit_grid: &'a [BitSet],
	next: Coord2D,
}
impl<'a> Iterator for WallIter<'a> {
    type Item = Coord2D;
    fn next(&mut self) -> Option<Self::Item> {
    	loop {
    		if self.next.x >= GameState::WIDTH {
    			self.next.x = 0;
    			self.next.y += 1;
    			if self.next.y >= GameState::HEIGHT {
    				return None;
    			}
    		}
    		let was = self.next;
    		self.next = Coord2D::new(was.x+1, was.y);
    		if self.bit_grid[was.y as usize].test(was.x as usize) {
    			return Some(was);
    		}
    	}
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
    	(0, None)
    }
}


#[derive(Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct Coord2D {
	pub x: u16,
	pub y: u16,	
}
impl Coord2D {
	#[inline]
	pub fn new(x: u16, y: u16) -> Self {
		Coord2D { x:x, y:y }
	}

	// ASSUMES ITS VALID
	pub fn move_with(&mut self, dir: Direction) {
		match dir {
			Direction::Up => self.y -= 1,
			Direction::Down => self.y += 1,
			Direction::Left => self.x -= 1,
			Direction::Right => self.x += 1,
		}
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub enum Direction {
	Left, Right, Up, Down,
}

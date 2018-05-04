use rand::{
	thread_rng,
	Rng,
};

#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct Moniker(pub char);

pub type ValidMove = bool;

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct GameState {
	monikers: Vec<(Coord2D, Moniker)>, 
}
impl GameState {
	const WIDTH: u16 = 16;
	const HEIGHT: u16 = 16;

	pub fn new() -> Self {
		GameState {
			monikers: vec![],
		}
	}

	pub fn contains_moniker(&self, moniker: Moniker) -> bool {
		self.find_moniker_index(moniker).is_none()
	}

	pub fn random_free_spot(&self) -> Option<Coord2D> {
		if self.monikers.len()/2 >= (Self::WIDTH * Self::HEIGHT) as usize {
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
			if self.find_coord_index(coord).is_none() {
				return Some(coord);
			}
		}
	}

	pub fn try_put_moniker(&mut self, moniker: Moniker, coord: Coord2D) -> ValidMove {
		if self.find_coord_index(coord).is_some() {
			return false;
		}
		self.monikers.push((coord, moniker));
		true
	}

	fn find_coord_index(&self, coord: Coord2D) -> Option<usize> {
		for (i, &(c, _m)) in self.monikers.iter().enumerate() {
			if c == coord {
				return Some(i)
			}
		}
		None
	}

	fn find_moniker_index(&self, moniker: Moniker) -> Option<usize> {
		for (i, &(_c, m)) in self.monikers.iter().enumerate() {
			if m == moniker {
				return Some(i)
			}
		}
		None
	}

	pub fn move_moniker_in_dir(&mut self, moniker: Moniker, dir: Direction) -> ValidMove {
		if let Some(index) = self.find_moniker_index(moniker) {
			let &mut (mut c, _p) = &mut self.monikers[index];
			if !Self::can_move_at(c, dir) {
				return false; // would move out of bounds
			}
			c.move_with(dir);
			true
		} else {
			false // no such moniker
		}
	} 

	pub fn can_move_at(coord: Coord2D, dir: Direction) -> bool {
		match dir {
			Direction::Up => coord.y > 0,
			Direction::Down => coord.y < Self::HEIGHT-1,
			Direction::Left => coord.x > 0,
			Direction::Right => coord.x < Self::WIDTH-1,
		}
	}

	pub fn draw(&self) {
		println!("===================");
		for y in 0..Self::HEIGHT {
			for x in 0..Self::WIDTH {
				let c = Coord2D::new(x, y);
				if let Some(index) = self.find_coord_index(c) { //NAIVE solution for now
					print!("{:?}", self.monikers[index].1);
				} else {
					print!(" ", );
				}
				
			}
			println!();
		}
	}
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct Coord2D {
	x: u16,
	y: u16,	
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
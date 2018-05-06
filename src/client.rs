use ::common::*;
use ::game::*;

use std::{
	net::SocketAddr,
	time::{
		Instant,
		Duration,
	},
	collections::HashMap,
};

use middleman::Middleman;

use mio::{
    Poll,
    Ready,
    PollOpt,
    Events,
    Token,
};

use ggez::{
    Context,
    GameResult,
    conf,
    graphics::{
        self,
        DrawMode,
        Point2,
        Mesh,
    },
    event::{
        self,
        Keycode,
        Mod,
    },
};

type TextCache = HashMap<Moniker, graphics::Text>;

const CLIENT_TOKEN: Token = Token(0);

pub fn client_enter(addr: &SocketAddr, my_moniker: Moniker) {
	println!("Client starting, for server at addr {:?}!", addr);
	match StdStream::connect(addr) {
		Ok(stream) => {
			stream.set_nodelay(true).expect("set_nodelay call failed");
			let mm = Middleman::new(MioStream::from_stream(stream).unwrap());
			client_go(mm, my_moniker);
		},
		Err(e) => {
			println!("Failed to connect to addr `{:?}`. Got err {:?}", addr, e);
		}
	}
}

fn client_go(mut mm: Middleman, my_moniker: Moniker) {
	let poll = Poll::new().unwrap();
	let mut events = Events::with_capacity(128);
	poll.register(&mm, CLIENT_TOKEN,
    			Ready::readable(),
    			PollOpt::edge()).unwrap();

	mm.send(& Serverward::Hello(my_moniker)).expect("HELLO send fail");
	use common::Clientward::*;
	let game_state = match mm.recv_blocking_solo::<Clientward>(&poll, &mut events, None).expect("crash").unwrap() {
		Welcome(essence) => GameState::from_essence(essence),
		msg => {
			println!("Got unexpected server msg {:?}", msg);
			panic!("Server Hello went awry");
		},
	};
	println!("Initial game state {:?}", &game_state);

	let c = conf::Conf::new();
    let ctx = &mut Context::load_from_conf("super_simple", "ggez", c).unwrap();

    let mut text_cache = HashMap::new();
    for &(_, moniker) in game_state.moniker_iter() {
    	insert_into_cache(ctx, &mut text_cache, moniker);
    }
    insert_into_cache(ctx, &mut text_cache, my_moniker);
    let (w, h) = graphics::get_size(ctx);
    let mut cs = ClientState {
        game_state: game_state,
        mm: mm,
        screen_dims: [w, h],
        poll: poll,
        events: events,
        mesh: build_square_mesh(ctx).unwrap(),
        poll_timeout: Some(Duration::from_millis(0)),
        no_change: false,
        text_cache: text_cache,
        last_move_at: Instant::now(),
    };
    event::run(ctx, &mut cs).unwrap();
}

fn build_square_mesh(ctx: &mut Context) -> GameResult<Mesh> {
	let mb = &mut graphics::MeshBuilder::new();
    let (w, h) = graphics::get_size(ctx);
    let w1 = (w / GameState::WIDTH as u32) as f32;
    let h1 = (h / GameState::HEIGHT as u32) as f32;
    mb.polygon(
    	DrawMode::Fill,
    	&[
    		Point2::new(0.0, 0.0),
    		Point2::new(w1, 0.0),
    		Point2::new(w1, h1),
    		Point2::new(0.0, h1),
    	],
    );
    mb.build(ctx)
}

fn insert_into_cache(ctx: &mut Context, text_cache: &mut TextCache, moniker: Moniker) {
	text_cache.insert(
    	moniker,
    	graphics::Text::new(
    		ctx,
    		& format!("{}", moniker.0),
    		& graphics::Font::default_font().unwrap()
    	).unwrap(),
    );
}

struct ClientState {
	screen_dims: [u32; 2],
    game_state: GameState,
    mm: Middleman,
    poll: Poll,
    events: Events,
    mesh: Mesh,
    poll_timeout: Option<Duration>,
    no_change: bool,
    text_cache: TextCache,
    last_move_at: Instant,
}
impl ClientState {
	fn translate(&self, coord: Coord2D) -> Point2 {
		Point2::new(
			(self.screen_dims[0] * coord.x as u32 / GameState::WIDTH as u32) as f32,
			(self.screen_dims[1] * coord.y as u32 / GameState::HEIGHT as u32) as f32,
		)
	}
}

impl event::EventHandler for ClientState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.poll.poll(&mut self.events, self.poll_timeout).expect("poll failed");
        if self.events.is_empty() {
            return Ok(());
        }
        self.no_change = false;
        self.events.clear();
        use self::Clientward::*;
        let (gs, mm, tx_cache) = (&mut self.game_state, &mut self.mm, &mut self.text_cache); 
        mm.recv_all_map( |_me, msg| {
        	println!("got {:?} from server", &msg);
            match msg {
                Welcome(_) => panic!("Not expecting a welcome"),
                AddPlayer(moniker, coord) => {
    				insert_into_cache(ctx, tx_cache, moniker);
                    gs.try_put_moniker(moniker, coord);
                },
                RemovePlayer(moniker) => {
                	tx_cache.remove(&moniker);
                    gs.try_remove_moniker(moniker);
                },
                UpdMove(moniker, dir) => {
                    gs.move_moniker_in_dir(moniker, dir);
                },
                some_err => {
                    println!("Server sent err msg {:?}", some_err);
                    panic!("server sent err");
                },
            };
        }).1.expect("Failed to read from server!");
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
    	let mut mv = |dir| {
    		if self.last_move_at.elapsed() < MOVE_PERIOD {
    			println!("I'm moving too fast!");
    			return;
    		}
    		self.last_move_at = Instant::now();
			self.mm.send(& Serverward::ReqMove(dir))
            .expect("req fail")
    	};
        match keycode {
        	Keycode::A |
            Keycode::Left => mv(Direction::Left),

        	Keycode::D |
            Keycode::Right => mv(Direction::Right),
            
        	Keycode::W |
            Keycode::Up => mv(Direction::Up),
            
        	Keycode::S |
            Keycode::Down => mv(Direction::Down),
            
            Keycode::Escape => ctx.quit().unwrap(),
            _ => (),
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
    	if self.no_change {
    		return Ok(());
    	}
        graphics::clear(ctx);
    	for &(coord, moniker) in self.game_state.moniker_iter() {
    		let moniker_text = self.text_cache.get(&moniker).unwrap();
    		let screen_point = self.translate(coord);
    		let param = graphics::DrawParam {
    			dest: screen_point, .. Default::default()
    		};
        	graphics::set_color(ctx, (255, 255, 255).into())?;
    		graphics::draw_ex(ctx, &self.mesh, param)?;
        	graphics::set_color(ctx, (0, 0, 0).into())?;
    		graphics::draw_ex(ctx, moniker_text, param)?;
    	}
    	graphics::set_color(ctx, (40, 0, 0).into())?;
    	for coord in self.game_state.coord_iter()
    	.filter(|&coord| self.game_state.is_wall_at(coord)) {
    		let screen_point = self.translate(coord);
    		let param = graphics::DrawParam {
    			dest: screen_point, .. Default::default()
    		};
    		graphics::draw_ex(ctx, &self.mesh, param)?;
    	}
        graphics::present(ctx);
        self.no_change = true;
        Ok(())
    }
}

use ::common::*;
use ::game::*;

use middleman::{
    Middleman,
    Message,
    PackedMessage,
};


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
    },
    event::{
        self,
        Keycode,
        Mod,
        EventHandler,
    },
};

use ::std::time::Duration;

const CLIENT_TOKEN: Token = Token(0);

struct ClientState {
    game_state: GameState,
    mm: Middleman,
    my_moniker: Moniker,
    poll: Poll,
    events: Events,
    poll_timeout = Option<Duration>,
}


impl event::EventHandler for ClientState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult<()> {
        self.poll(&mut self.events, self.short_timeout);
        if self.events.is_empty() {
            return;
        }
        self.events.clear();
        use Clientward::*;
        self.mm.recv_all_map( |me: &mut Middleman, msg| {
            match msg {
                Welcome(_) => (),
                AddPlayer(moniker, coord) => {
                    self.game_state.try_put_moniker(moniker, coord);
                },
                RemovePlayer(moniker) => {
                    self.game_state.try_remove_moniker(moniker);
                },
                UpdMove(moniker, dir) => {
                    self.game_state.move_moniker_in_dir(moniker, dir);
                },
                some_err => {
                    println!("Server sent err msg {:?}", some_err);
                },
            }
        }).1.expect("Failed to read from server!");
        Ok(())
    }

    fn key_down_event(&mut self, ctx: &mut Context, keycode: Keycode, _keymod: Mod, _repeat: bool) {
        match keycode {
            Keycode::Left => {
                self.mm.send(& Serverward::ReqMove(Direction::Left))
                .expect("req fail")
            },
            Keycode::Right => {
                self.mm.send(& Serverward::ReqMove(Direction::Right))
                .expect("req fail")
            },
            Keycode::Up => {
                self.mm.send(& Serverward::ReqMove(Direction::Up))
                .expect("req fail")
            },
            Keycode::Down => {
                self.mm.send(& Serverward::ReqMove(Direction::Down))
                .expect("req fail")
            },
            Keycode::Escape => ctx.quit().unwrap(),
            _ => (),
        }
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // graphics::clear(ctx);
        // graphics::circle(ctx,
        //                  DrawMode::Fill,
        //                  Point2::new(self.pos_x, 380.0),
        //                  100.0,
        //                  2.0)?;
        // graphics::present(ctx);
        Ok(())
    }
}

pub fn gg_main(game_state: GameState, my_moniker: Moniker, mm: Middleman) {
    let c = conf::Conf::new();
    let ctx = &mut Context::load_from_conf("super_simple", "ggez", c).unwrap();
    let cs = ClientState {
        game_state: game_state,
        mm: mm,
        my_moniker: my_moniker,
        poll: Poll::new().unwrap(),
        events: Events::with_capacity(128),
        poll_timeout: Some(Duration::from_millis(30)),
    };
    event::run(ctx, &mut cs).unwrap();
}
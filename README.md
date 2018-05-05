# Pinggame

This is a little "game" I made for testing ping for a stateful game like this. It's hardly a game, but its quite extensible, and I have some helper libs planned to simplify it even further (particularly server-side).

## Setup
1. Ensure you have the stable branch of [rust](https://www.rust-lang.org/) installed. Use [rustup](https://rustup.rs) to install the latest 'stable' toolchain.
1. clone this [repo](https://github.com/sirkibsirkib/pinggame) with [git](https://git-scm.com/). Eg: type `git clone https://github.com/sirkibsirkib/pinggame` into your terminal.
1. Follow everything after (not including) the 'project setup' section of [these instructions](https://github.com/ggez/ggez/blob/master/docs/BuildingForEveryPlatform.md) to setup [SDL2](https://www.libsdl.org/) on your machine. (the underlying `ggez` game engine relies on it).
1. run `cargo build --release` inside the cloned repo. If its takes a moment, that's a good sign. If all goes well, you'll see something like this:
```
$ cargo build --release
		... a lot of stuff ...
   Compiling rodio v0.6.0
   Compiling image v0.18.0
   Compiling gfx v0.17.1
   Compiling gfx_device_gl v0.15.2
   Compiling gfx_window_sdl v0.8.0
   Compiling ggez v0.4.2
   Compiling pinggame v0.1.0 (file:///C:/Git/pinggame)
    Finished release [optimized] target(s) in 270.70 secs
$
```
1. If it doesn't go well, you likely need to relook the SDL2 step above.

## Playing
Once you have your executable (called `pinggame` or `pinggame.exe`), you can run it in your terminal.

### Server mode
If you want to be the server, run:
```
./pinggame "127.0.0.1:9000" 
```
the first argument is your ip and port number. This is where the clients will find you. If you want to be discoverable beyond your local network, look into 'public IP' and 'port forwarding'.

### Client mode
If you want to be a client (player), run:
```
./pinggame "127.0.0.1:9000" -m "Q"
```
The first argument is where the game will try and find the server. If it fails to connect, check if the server is running, the IP is correct, your firewall isnt causing trouble (on _either_ end), possibly your port forwarding, and you have considered public vs private IP.

The second argument is you setting a flag `-m` to indicate client mode by providing your in-game _(m)oniker_. The moniker itself follows the flag, and must simply be _any ascii character_ (unicode isn't displayed correctly by `ggez`).


# The game
You're a square with your moniker as a label. Use `WASD` or the arrowkeys to move around. Have a _blast_.
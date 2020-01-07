#![feature(process_exitcode_placeholder)]
#![feature(clamp)]
mod texture;

use asnet::{self, EventKind, Host};
use proto::{ClientMessage, Position, ServerMessage};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use texture::Texture;

const SLEEP: Duration = Duration::from_millis(1000 / 60);
const PLAYER_SPEED: f64 = 4.0;
const SHOT_SPEED: f64 = 8.0;
const SHOT_TTL: u8 = 2;

struct State {
    id: u32,
    health: u8,
    moving: Moving,
    rotating: Rotating,
    position: Position,
    players: HashMap<u32, Position>,
    shots: Vec<Shot>,
}

enum Moving {
    Up,
    Down,
    Nowhere,
}

enum Rotating {
    Left,
    Right,
    Nowhere,
}

struct Shot {
    ttl: u8,
    position: Position,
}

fn run(addr: SocketAddr) -> Result<(), Error> {
    let sdl = sdl2::init()?;
    let mut canvas = sdl
        .video()?
        .window(env!("CARGO_PKG_NAME"), 0, 0)
        .opengl()
        .hidden()
        .build()?
        .into_canvas()
        .build()?;
    let mut event_pump = sdl.event_pump()?;

    let creator = canvas.texture_creator();
    let player_texture = Texture::new(&creator, include_bytes!("../data/player.png"))?;
    let charged_shot_texture = Texture::new(&creator, include_bytes!("../data/charged_shot.png"))?;
    let discharged_shot_texture =
        Texture::new(&creator, include_bytes!("../data/discharged_shot.png"))?;
    let health_texture = Texture::new(&creator, include_bytes!("../data/health.png"))?;

    let mut host = Host::<()>::client()?;
    let idx = host.connect(addr)?.idx();

    let mut state = None;
    let mut tick = Instant::now();
    'main: loop {
        if let Some(event) = host.process(Duration::from_secs(0))? {
            match event.kind {
                EventKind::Connect => {}
                EventKind::Disconnect => return Err("Disconnected from server".into()),
                EventKind::Receive(packet) => match bincode::deserialize(&packet)? {
                    ServerMessage::Init { id, width, height } => {
                        state = Some(State {
                            id,
                            health: 100,
                            moving: Moving::Nowhere,
                            rotating: Rotating::Nowhere,
                            position: Position {
                                x: width as f64 / 2.0,
                                y: height as f64 / 2.0,
                                angle: 0.0,
                            },
                            players: HashMap::new(),
                            shots: Vec::new(),
                        });

                        let window = canvas.window_mut();
                        window.set_size(width, height)?;
                        window.show();
                    }
                    ServerMessage::Move { id, position } => {
                        let state = state.as_mut().ok_or("Unexpected move message")?;

                        state.players.insert(id, position);
                    }
                    ServerMessage::Shoot { id } => {
                        let state = state.as_mut().ok_or("Unexpected move message")?;

                        if let Some(position) = state.players.get(&id).copied() {
                            let angle = position.angle.to_radians();

                            state.shots.push(Shot {
                                ttl: SHOT_TTL,
                                position: Position {
                                    x: position.x
                                        + (player_texture.width() / 2
                                            - discharged_shot_texture.width() / 2)
                                            as f64
                                        + angle.sin() * player_texture.width() as f64,
                                    y: position.y + discharged_shot_texture.height() as f64 / 2.0
                                        - angle.cos() * player_texture.height() as f64,
                                    angle: position.angle,
                                },
                            });
                        }
                    }
                    ServerMessage::Leave { id } => {
                        let state = state.as_mut().ok_or("Unexpected leave message")?;

                        state.players.remove(&id);
                    }
                },
            }
        }

        for event in event_pump.poll_iter() {
            if let Event::Quit { .. } = event {
                break 'main;
            }

            if let Some(ref mut state) = state {
                match event {
                    Event::KeyDown {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Left => state.rotating = Rotating::Left,
                        Keycode::Right => state.rotating = Rotating::Right,
                        Keycode::Up => state.moving = Moving::Up,
                        Keycode::Down => state.moving = Moving::Down,
                        Keycode::Space => {
                            host[idx].send(bincode::serialize(&ClientMessage::Shoot).unwrap());
                        }
                        Keycode::S => {
                            host[idx].send(bincode::serialize(&ClientMessage::Die).unwrap());
                        }
                        _ => {}
                    },
                    Event::KeyUp {
                        keycode: Some(keycode),
                        ..
                    } => match keycode {
                        Keycode::Left | Keycode::Right => state.rotating = Rotating::Nowhere,
                        Keycode::Up | Keycode::Down => state.moving = Moving::Nowhere,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        let now = Instant::now();
        if now - tick < SLEEP {
            continue;
        }

        tick = now;

        let state = match state.as_mut() {
            Some(state) => state,
            None => continue,
        };

        host[idx].send(
            bincode::serialize(&ClientMessage::Move {
                position: state.position,
            })
            .unwrap(),
        );

        let (width, height) = canvas.window().size();

        canvas.set_draw_color(Color::RGB(32, 32, 32));
        canvas.clear();

        match state.moving {
            Moving::Up => {
                let angle = state.position.angle.to_radians();

                state.position.x += angle.sin() * PLAYER_SPEED;
                state.position.y -= angle.cos() * PLAYER_SPEED;
            }
            Moving::Down => {
                let angle = state.position.angle.to_radians();

                state.position.x -= angle.sin() * PLAYER_SPEED;
                state.position.y += angle.cos() * PLAYER_SPEED;
            }
            Moving::Nowhere => {}
        }

        match state.rotating {
            Rotating::Left => state.position.angle = (state.position.angle - PLAYER_SPEED) % 360.0,
            Rotating::Right => state.position.angle = (state.position.angle + PLAYER_SPEED) % 360.0,
            Rotating::Nowhere => {}
        }

        canvas.copy(
            health_texture.inner(),
            Rect::new(
                0,
                0,
                ((state.health as f64 / 100.0) * width as f64) as u32,
                health_texture.height(),
            ),
            Rect::new(
                0,
                (height - health_texture.height()) as i32,
                ((state.health as f64 / 100.0) * width as f64) as u32,
                health_texture.height(),
            ),
        )?;

        for shot in &mut state.shots {
            let texture = if shot.ttl != SHOT_TTL {
                &charged_shot_texture
            } else {
                &discharged_shot_texture
            };

            canvas.copy_ex(
                texture.inner(),
                None,
                Rect::new(
                    shot.position.x as i32,
                    shot.position.y as i32,
                    texture.width(),
                    texture.height(),
                ),
                shot.position.angle,
                None,
                false,
                false,
            )?;

            let angle = shot.position.angle.to_radians();
            shot.position.x += angle.sin() * SHOT_SPEED;
            shot.position.y -= angle.cos() * SHOT_SPEED;

            if shot.position.x > width as f64 || shot.position.x < 0.0 {
                shot.position.x = shot.position.x.clamp(0.0, width as f64);
                shot.position.angle = 360.0 - shot.position.angle;
                shot.ttl = shot.ttl.saturating_sub(1);
            }

            if shot.position.y > height as f64 || shot.position.y < 0.0 {
                shot.position.y = shot.position.y.clamp(0.0, height as f64);
                shot.position.angle = (180.0 - shot.position.angle) % 360.0;
                shot.ttl = shot.ttl.saturating_sub(1);
            }

            if shot.ttl == SHOT_TTL {
                continue;
            }

            for (id, position) in &state.players {
                if shot.position.x >= position.x
                    && shot.position.x <= position.x + player_texture.width() as f64
                    && shot.position.y >= position.y
                    && shot.position.y <= position.y + player_texture.height() as f64
                {
                    shot.ttl = 0;

                    if *id == state.id {
                        state.health = state.health.saturating_sub(1);
                        if state.health == 0 {
                            host[idx].send(bincode::serialize(&ClientMessage::Die).unwrap());
                        }
                    }
                }
            }
        }

        state.shots.retain(|shot| shot.ttl != 0);

        for (_, position) in &state.players {
            canvas.copy_ex(
                player_texture.inner(),
                None,
                Rect::new(
                    position.x as i32,
                    position.y as i32,
                    player_texture.width(),
                    player_texture.height(),
                ),
                position.angle,
                None,
                false,
                false,
            )?;
        }

        canvas.present();
    }

    Ok(())
}

#[derive(StructOpt)]
struct Args {
    addr: SocketAddr,
}

fn main() -> ExitCode {
    let args = match Args::from_iter_safe(env::args()) {
        Ok(args) => args,
        Err(err) => {
            println!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    if let Err(Error(err)) = run(args.addr) {
        println!("Error: {}", err);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

struct Error(String);

impl<T> From<T> for Error
where
    T: std::fmt::Display,
{
    fn from(err: T) -> Error {
        Error(err.to_string())
    }
}

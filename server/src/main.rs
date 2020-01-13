use asnet::{Event, EventKind, Host};
use proto::{ClientMessage, ServerMessage};
use slab::Slab;
use std::env;
use std::io::Error;
use std::net::SocketAddr;
use std::process;
use structopt::StructOpt;

fn run(addr: SocketAddr, width: u32, height: u32) -> Result<(), Error> {
    let mut host = Host::<u32>::server(addr)?;
    let mut players = Slab::new();
    loop {
        let Event { kind, peer } = host.process_blocking()?;
        match kind {
            EventKind::Connect => {
                println!("{}: connected", peer.addr());

                *peer.data_mut() = players.insert(()) as u32;

                peer.send(
                    bincode::serialize(&ServerMessage::Init {
                        id: *peer.data(),
                        width,
                        height,
                    })
                    .unwrap(),
                );
            }
            EventKind::Disconnect => {
                println!("{}: disconnected", peer.addr());

                let id = *peer.data();
                players.remove(id as usize);

                host.broadcast(bincode::serialize(&ServerMessage::Leave { id }).unwrap());
            }
            EventKind::Receive(packet) => {
                let message = match bincode::deserialize(&packet) {
                    Ok(message) => message,
                    Err(err) => {
                        println!("{}: error deserializing message: {}", peer.addr(), err);
                        peer.disconnect();
                        continue;
                    }
                };

                let id = *peer.data();
                let message = match message {
                    ClientMessage::Move { position } => ServerMessage::Move { id, position },
                    ClientMessage::Shoot => ServerMessage::Shoot { id },
                    ClientMessage::Die => {
                        peer.send(
                            bincode::serialize(&ServerMessage::Init { id, width, height }).unwrap(),
                        );
                        continue;
                    }
                };

                host.broadcast(bincode::serialize(&message).unwrap());
            }
        }
    }
}

#[derive(StructOpt)]
struct Args {
    listen_addr: SocketAddr,
    width: u32,
    height: u32,
}

fn main() {
    let ok = (|| {
        let args = match Args::from_iter_safe(env::args()) {
            Ok(args) => args,
            Err(err) => {
                println!("{}", err);
                return false;
            }
        };

        if let Err(err) = run(args.listen_addr, args.width, args.height) {
            println!("Error: {}", err);
            return false;
        }

        true
    })();

    process::exit(!ok as i32);
}

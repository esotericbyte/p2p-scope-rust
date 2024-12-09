// Copyright 2023 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

// TODO: Write up instructions
//! A chat application demonstrating libp2p with the mDNS and floodsub protocols
//! using tokio for all asynchronous tasks and I/O. In order for all used libp2p
//! crates to use tokio, it enables tokio-specific features for some crates.
//!
//! The example is run per node as follows:
//!
//! ```sh
//! cargo run --example chat-tokio --features=full
//! ```

#![doc = include_str!("../README.md")]
mod cursive_tui;

use crate::cursive_tui::{UiUpdate, CursiveCallback,
                         ui_update_to_cursive_callback,
                         terminal_user_interface};
// Lib p2p and related includes


use std::{
    collections::hash_map::DefaultHasher,
    error::Error,
    hash::{Hash, Hasher},
    time::Duration,
};

use futures::stream::StreamExt;
use libp2p::{
    identity,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux,
    PeerId,
};

use std::error::Error;
use std::str::FromStr;
use tokio;
use tokio::io::AsyncBufReadExt;
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;
// Command line arguments defined for clap at the end of this file
use clap::Parser;
use cursive::CbSink;
use libp2p::swarm::KeepAlive;
use tokio::tracing_subscriber
// custom network behavior so it can be exended
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    // I dislike! mdns: mdns::tokio::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
// let _ = tracing_subscriber::fmt()
//         .with_env_filter(EnvFilter::from_default_env())
//         .try_init();    

    //parse command line arguments

    let clap_args = CliArguments::parse();
    let args_text = format!("cli args: {:?}", clap_args);

    // Initialize Lib-p2p instance information
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());


    // Stage the channels and functions used to communicate between tokio and the UI thread
    let (input_sender,mut input_receiver) =
        tokio::sync::mpsc::channel::<Box<String>>(32);
    //let (update_sender,mut update_receiver) =
    //   std::sync::mpsc::channel::<Box<UiUpdate>>();
    let (cb_sync_sender,
        mut cb_sync_receiver) = tokio::sync::oneshot::channel();
    let clap_args_clone = clap_args.clone(); //clone is a value to move
    // A regular sync thread running along side of the tokio runtime.
    let _tui_handle = std::thread::spawn(move || {
        terminal_user_interface(input_sender.clone(),
                                peer_id,
                                clap_args_clone,
                                cb_sync_sender);
    });

    let cb_sink = cb_sync_receiver.await.unwrap();// get callback channel from new thread

    let cb_sink_clone = cb_sink.clone();// avoid loosing cb_sink
    let terminal_output =move |output:String| {
        cb_sink_clone.send(ui_update_to_cursive_callback(
            UiUpdate::TerminalOutput(output))).unwrap();
    };

    let cb_sink_clone2 = cb_sink.clone();
    let send_ui_update = |tui_update:UiUpdate|{
        cb_sink_clone2.send(ui_update_to_cursive_callback(
            tui_update)).unwrap()};




    // Create a Swarm to manage peers and events.
    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
                // signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;
            Ok(MyBehaviour { gossipsub })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(120)))
        .build();

    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("monolith");
    // subscribes to our topic
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Reach out to another node if specified
    match clap_args.dial {
        Some(addr_list) => {
            for addr in addr_list {
                swarm.dial(addr.clone()).unwrap();
                (terminal_output)(format!("Dialed {:?}", addr));
            }
        }
        None => {
            (terminal_output)(format!("No addresses Dialed"));
        }
    }
    // Replaced by Tui
    // Read full lines from stdin
    // let mut stdin = io::BufReader::new(io::stdin()).lines();
    // Listen mode takes president over listen which can be given multiple times.
    // Listening on all networks is the default if neither are specified
    let all_nets_addr :Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    let localhost_addr :Multiaddr = "/ip4/127.0.0.1/tcp/0".parse()?;
    let mut no_listen = false;
    let mut default_listen_all= false;

    if let None = clap_args.listen_mode{
        if let Some(addrs_vec) = clap_args.listen {
            for addr in addrs_vec{
                swarm.listen_on(addr.clone())?;
            }
        } else {
            // no listen mode or specified addr/ port so default to all!
            default_listen_all = true;
            swarm.listen_on(all_nets_addr.clone())?;
        }
    }

    if let Some(ListenMode) = clap_args.listen_mode {
        match ListenMode {
            // Listen on all interfaces and whatever port the OS assigns
            ListenMode::All => {
                swarm.listen_on(all_nets_addr.clone())?;
            }
            ListenMode::DoNotListen => {
                (terminal_output)(format!("Not listening! La! La! La!"));
                no_listen = true;
            }
            ListenMode::Localhost => {
                swarm.listen_on(localhost_addr.clone())?;
            }
            // ListenMode::Lan => {
            //     swarm.listen_on(all_ports)?;
            //     (terminal_output)(format!("LAN limitation unimplemented"));
            //}
        }
    }

    let listeners = swarm.listeners();
    (terminal_output)("LISTENERS:\r".to_string());
    for ma in listeners {
        (terminal_output)(format!("{:?}\r", ma));
    }

    // handle network and UI events in the same loop
    loop {
        tokio::select! {
            Some(box_message) = input_receiver.recv() => {
                if let Err(e) = swarm
                    .behaviour_mut().gossipsub
                    .publish(topic.clone(), *box_message.as_bytes()) {
                    (terminal_output)(format!("Publish error: {e:?}"));
                }}
            //Todo:handle other messages, terminate message, topics, layout changes,
            //  event list, menubar, text commands.
            event = swarm.select_next_some() => match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        (terminal_output)(format!("Listening on {address:?}"));
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) =>(send_ui_update)(
                            UiUpdate::TextMessage( String::from("monolith"),
                                message.source,
                                String::from_utf8(&message.data))
                        );
                    SwarmEvent::ConnectionEstablished{peer_id,..} => {
                        (terminal_output)(format!("Connected!: '{:?}'",event));
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }

                    other_swarm_event => {
                        (terminal_output)(format!("EVENT: {:?}",other_swarm_event));
                    }
                }
            }
        }
    }

//old keep-alive code

//                    SwarmEvent::ConnectionClosed {
//                        peer_id,
//                        endpoint: ConnectedPoint::Dialer { address,.. },
//                        cause: Some(KeepAliveTimeout),..} => {
//                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
//                        // Hanging up so rude! Redial !
//                        // maybe a goodbye message. I believe this will only retry once.
//                       (terminal_output)(format!("KeepAliveTimeout, Redialing {:?}",address));
//                        swarm.dial(address)?;
//                    }
//                   SwarmEvent::ConnectionClosed {peer_id,..} =>{
//                       swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
//                      (terminal_output)(format!("CLOSED:{:?}", event));
//                    }

// Argument parsing initialization
#[derive(Parser, Default, Debug, Clone)]
#[clap(author = "John Hall", version, about)]
pub struct CliArguments {
    #[arg(long, value_enum)]
    /// Takes president over listen which can be given multiple times.
    /// Listening on all networks is the default if neither are specified
    listen_mode: Option<ListenMode>,
    /// Light ar dark theme can be picked. Default is light.
    theme: Option<Theme>,
    #[arg(long, value_parser = parse_and_transform_multiaddr)]
    /// Multiaddr to dial. --dial may be given multiple times.
    dial: Option<Vec<Multiaddr>>,
    /// Specify host network and port to listen on. May be given multiple times but is ignored
    /// if listen-mode is also given.
    listen: Option<Vec<Multiaddr>>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum Theme{
    Light,
    Dark
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum ListenMode {
    DoNotListen,
    All,
    Localhost,
    //Lan,
}

fn parse_and_transform_multiaddr(s: &str) -> Result<Multiaddr, String> {
    // Transform the address string here
    let transformed_addr = if s.starts_with('\\') {
        &s[1..]
    } else {
        s
    };

    // Now parse the transformed string as a Multiaddr
    Multiaddr::from_str(transformed_addr).map_err(|e| e.to_string())
}
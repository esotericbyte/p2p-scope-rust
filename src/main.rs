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


mod cursive_tui;

use crate::cursive_tui::{UiUpdate, ui_update_to_cursive_callback, terminal_user_interface, CursiveCallback};
// Lib p2p and related includes
use libp2p::core::{ConnectedPoint};
use libp2p::swarm::ConnectionError::KeepAliveTimeout;
pub(crate) use libp2p::{
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent},
    futures::StreamExt,
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, Multiaddr, PeerId, Transport,
};

use std::error::Error;
use tokio;
use tokio::io::AsyncBufReadExt;
use std::sync::mpsc;
// Command line arguments defined for clap at the end of this file
use clap::Parser;
use cursive::CbSink;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Init Tracing. For mostly using logging features 
    
    //
    //
    //parse command line arguments

    let clap_args = CliArguments::parse();
    let args_text = format!("cli args: {:?}", clap_args);

    // Initialize Lib-p2p instance information
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());


    // Stage the channels and functions used to communicate between tokio and the UI thread
    let (input_sender, input_receiver) =
        tokio::sync::mpsc::channel::<Box<String>>(32);
    let (update_sender, update_receiver) =
        std::sync::mpsc::channel::<Box<UiUpdate>>();
    let (cb_sync_sender,cb_sync_receiver) = tokio::sync::oneshot();


    // A regular sync thread running along side of the tokio runtime.
    let _tui_handle = std::thread::spawn(move || {
        terminal_user_interface(input_sender.clone(),
                                peer_id,
                                clap_args.clone(),
                                cb_sync_sender)
    });

    let cb_sink = cb_sync_receiver.recv();//get the callback channel from the new thread!

    let cb_sink_clone = cb_sink.clone();// avoid loosing cb_sink
    let terminal_output =move |output:String| {
        cb_sink_clone.send(ui_update_to_cursive_callback(
            UiUpdate::TerminalOutput(output))).unwrap();
    };

    let cb_sink_clone2 = cb_sink.clone();
    let send_ui_update = |tui_update:UiUpdate|{
        cb_sink_clone2.send(ui_update_to_cursive_callback(
            tui_update)).unwrap()};

    // Create a tokio-based TCP transport use noise for authenticated
    // encryption and Mplex for multiplexing of substreams on a TCP stream.
    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(
            noise::NoiseAuthenticated::xx(&id_keys)
                .expect("Signing libp2p-noise static DH keypair failed."),
        )
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Create a Floodsub topic.  Note changed in scope version.
    let floodsub_topic = floodsub::Topic::new("monolith");

    // We create a custom  behaviour that combines floodsub and mDNS.
    // The derive generates a delegating `NetworkBehaviour` impl.
    #[derive(NetworkBehaviour)]
    #[behaviour(out_event = "MyBehaviourEvent")]
    struct MyBehaviour {
        floodsub: Floodsub,
        mdns: mdns::tokio::Behaviour,
    }

    #[derive(Debug)]
    #[allow(clippy::large_enum_variant)]
    enum MyBehaviourEvent {
        Floodsub(FloodsubEvent),
        Mdns(mdns::Event),
    }

    impl From<FloodsubEvent> for MyBehaviourEvent {
        fn from(event: FloodsubEvent) -> Self {
            MyBehaviourEvent::Floodsub(event)
        }
    }

    impl From<mdns::Event> for MyBehaviourEvent {
        fn from(event: mdns::Event) -> Self {
            MyBehaviourEvent::Mdns(event)
        }
    }

    // Create a Swarm to manage peers and events.
    let mdns_behaviour = mdns::Behaviour::new(Default::default(), peer_id)?;
    let behaviour = MyBehaviour {
        floodsub: Floodsub::new(peer_id),
        mdns: mdns_behaviour,
    };
    let mut swarm = Swarm::with_tokio_executor(transport, behaviour, peer_id);
    swarm
        .behaviour_mut()
        .floodsub
        .subscribe(floodsub_topic.clone());

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

    let all_ports = "/ip4/0.0.0.0/tcp/0".parse()?;
    let local_port = "/ip4/127.0.0.0/tcp/0".parse()?;
    let mut no_listen = false;
    if let Some(ListenMode) = clap_args.listen_mode {
        match ListenMode {
            // Listen on all interfaces and whatever port the OS assigns
            ListenMode::All => {
                swarm.listen_on(all_ports)?;
            }
            ListenMode::NoListen => {
                (terminal_output)(format!("Not listening! La La la La La!"));
                no_listen = true;
            }
            ListenMode::Localhost => {
                swarm.listen_on(local_port)?;
            }
            ListenMode::Lan => {
                swarm.listen_on(all_ports)?;
                (terminal_output)(format!("LAN limitation unimplemented"));
            }
        }
    }
    let listeners = swarm.listeners();
    (terminal_output)("LISTENERS:\r".to_string());
    for ma in listeners {
        (terminal_output)(format!("{:?}\r", ma));
    }

    if let Some(maddr) = clap_args.listen_on {
        if !no_listen {
            swarm.listen_on(maddr)?;
        }
    }

    // Kick it off
    loop {
        tokio::select! {
            Some(box_message) = input_receiver.recv() => {
                let message = *box_message;
                swarm.behaviour_mut().floodsub.publish_any(
                    floodsub_topic.clone(), message);
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        (terminal_output)(format!("Listening on {address:?}"));
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(
                        FloodsubEvent::Message(message))) => {
                        let message_string = String::from_utf8(message.data).unwrap();
                        let t =message.topics;
                        // ToDo: learn about the Vec<Topics> part of the Floodsub message. convert.
                        (send_ui_update)(
                            UiUpdate::TextMessage( String::from("monolith"),
                                message.source,
                                message_string)
                        );
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(event)) => {
                        match event {
                            mdns::Event::Discovered(list) => {
                                for (peer, _) in list {
                                    swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer);
                                }
                            }
                            mdns::Event::Expired(list) => {
                                for (peer, _) in list {
                                    if !swarm.behaviour().mdns.has_node(&peer) {
                                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer);
                                    }
                                }
                            }
                        }
                    }
                    SwarmEvent::ConnectionEstablished{peer_id,..} => {
                        (terminal_output)(format!("Connected!: '{:?}'",event));
                        swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    }
                    SwarmEvent::ConnectionClosed {peer_id, endpoint: ConnectedPoint::Dialer { address,.. },
                        cause: Some(KeepAliveTimeout),..} => {
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        // Hanging up so rude! Redial !
                        // maybe a goodbye message. I believe this will only retry once.
                        (terminal_output)(format!("KeepAliveTimeout, Redialing {:?}",address));
                        swarm.dial(address)?;
                    }
                    SwarmEvent::ConnectionClosed {peer_id,..} =>{
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        (terminal_output)(format!("DRAT!:{:?}", event));
                    }
                    other_swarm_event => {
                        (terminal_output)(format!("EVENT: '{:?}'",other_swarm_event));
                    }
                }
            }
        }
    }
}

// Argument parsing initialization
#[derive(Parser, Default, Debug, Clone)]
#[clap(author = "John Hall et. al.", version, about)]
pub(crate) struct CliArguments {
    #[arg(long, value_enum)]
    listen_mode: Option<ListenMode>,
    theme: Option<Theme>,
    #[arg(long)]
    dial: Option<Vec<Multiaddr>>,
    listen_on: Option<Multiaddr>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum Theme{
    Light,
    Dark
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub(crate) enum ListenMode {
    All,
    Localhost,
    Lan,
    NoListen,
}


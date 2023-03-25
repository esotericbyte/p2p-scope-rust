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

//! A basic chat application demonstrating libp2p with the mDNS and floodsub protocols
//! using tokio for all asynchronous tasks and I/O. In order for all used libp2p
//! crates to use tokio, it enables tokio-specific features for some crates.
//!
//! The example is run per node as follows:
//!
//! ```sh
//! cargo run --example chat-tokio --features=full
//! ```

// Lib p2p and related includes
use libp2p::core::{ConnectedPoint, Endpoint};
use libp2p::swarm::ConnectionError::KeepAliveTimeout;
use libp2p::{
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent},
    futures::StreamExt,
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
    tcp, Multiaddr, PeerId, Transport,
};

use std::error::Error;
use std::io::Read;
use std::ptr::addr_of_mut;
use std::sync::mpsc;
use std::sync::mpsc::{SendError, Sender};
use tokio;
use tokio::io::AsyncBufReadExt;

// TUI and argument parsing includes
use clap::Parser;
use cursive;
use cursive::backends::crossterm::crossterm::style;
use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::event::Event::Refresh;
use cursive::reexports::toml::to_string;
use cursive::theme::Effect::Bold;
use cursive::theme::*;
use cursive::traits::*;
use cursive::utils::*;
use cursive::view::{Nameable, Position, Scrollable};
use cursive::views::{
    Button, Dialog, EditView, LinearLayout, ListView, Panel, ResizedView, ScrollView, TextView,
};
use cursive::{CbSink, Cursive};
use tokio::sync::oneshot;

#[derive(Debug)]
enum TuiUpdate {
    // topic , from_id , message
    AppendMessage(String, String, String),
    NewContent(String, String),
}

// cursive allows to store a user data in it's runtime so this struct is for that purpose
struct TheUserData {
    user_message_sender: tokio::sync::mpsc::Sender<Box<String>>,
    libp2p_network_id: String,
    command_line_opts: String,
}
//tui_update_receiver: std::sync::mpsc::Receiver<Box<TuiUpdate>>,

// Cursive  UI has 2 phases
// In the first phase the UI is declared
// In the second phase it is run on an event loop in a standard syncronous thread.
//
// The primary way it to communicate into the thread during the run phase is a
// callback sync which i think is an mpsc channel under the hood.
// Callbacks in the form of closures are the primary way to send changes to the runtime.
// There is a 'user data' instance within the thread and I've used this to send data out of the
// Thread to the tokio runtime.
//
// Tokio runtime is completely different from cursive. They are 2 separate event loops and will be
// run in separate threads.
//
// Current plan is for Cursive to run in a standard thread and for tokio in the main thread running
// the tokio runtime defined by the tokio main macro.
//
// This next function will be run in a separate thread.
// To change the ui, the callback sync provided by Cursive is sent by a oneshot channel.
//
//
fn terminal_user_interface(
    user_message_sender: tokio::sync::mpsc::Sender<Box<String>>,
    cb_sync_sender: oneshot::Sender<Box<&CbSink>>,
    libp2p_network_id: String,
    command_line_opts: String,
)
// async send back: cb_sync
// &Sender<Box<dyn FnOnce(&mut Cursive) + Send + 'static, Global>>
{
    // Initialize Cursive TUI
    let mut siv = Cursive::new();
    let cursive_call_back_sink: Box<&CbSink> = Box::new(siv.cb_sink());
    cb_sync_sender.send(cursive_call_back_sink).unwrap();
    //dark color scheme
    siv.load_toml(include_str!("colors.toml")).unwrap();

    let user_data = TheUserData {
        user_message_sender,
        libp2p_network_id,
        command_line_opts,
    };

    siv.set_user_data(user_data); //value as &dyn Any
    siv.add_global_callback(
        cursive::event::Event::CtrlChar('d'),
        move |s: &mut Cursive| {
            s.toggle_debug_console();
        },
    );
    //    siv.add_global_callback(Refresh, move |s: &mut Cursive| {
    // other direction        channel_result = s.user_data()::<TheUserData>.user_message_sender

    // CURSIVE  TUI views
    //let peers_view
    //let ports_view
    let user_message_input = LinearLayout::new(Horizontal)
        .child(
            EditView::new()
                .on_submit(new_user_message)
                .with_name("user_message_input")
                .min_width(40),
        )
        .child(Button::new("Send", |s: &mut Cursive| {
            s.call_on_name("user_message_input", |v: &mut EditView| {
                v.set_content("foo var")
            });
        }));

    // let user_message_history = ListView::new()
    //    .with_name("user_message_history")
    //    .on_select(selected_message);

    let monolith_chat_view = TextView::new("MONOLITH CHAT\r")
        .with_name("monolith_chat_view")
        .min_width(20)
        .max_height(16)
        .scrollable();
    let output_view = TextView::new("OUTPUT VIEW\r")
        .with_name("output_view")
        .min_width(20)
        .min_height(10)
        .max_height(16)
        .scrollable();

    let instance_info_view =
        TextView::new(format!("peer id: {} cli args:{}",libp2p_network_id, command_line_opts))
        .with_name("instance_info")
        .full_width()
        .min_height(2);

    // let peers_and_ports_layout = LinearLayout::horizontal.new()
    //    .child(peers_view)
    //    .child(ResizedView.with_percent_width(20).child(ports_view))
    let scope_screen = ResizedView::with_full_screen(
        LinearLayout::vertical()
            .child(instance_info_view)
            //.child(peers_and_ports)
            .child(user_message_input)
            //.child(user_message_history)
            .child(
                LinearLayout::new(Horizontal)
                    .child(monolith_chat_view)
                    .child(output_view),
            ),
    );
    siv.add_layer(scope_screen);
    siv.run();
}
// inital callbacks
fn new_user_message(s: &mut Cursive, message: &str) {
    s.call_on_name("monolith_chat_view", |v: &mut TextView| {
        v.append(format!("{}\r", message))
    });
    s.call_on_name("user_message_input", |v: &mut EditView| v.set_content(""));
    let ud: &TheUserData = s.user_data().unwrap();
    ud.user_message_sender
        .blocking_send(Box::new(message.to_string()))
        .unwrap();
    //TODO: add messages to history
}

// CURSIVE TUI Functions
fn dlg_on_quit(s: &mut Cursive) {
    s.add_layer(
        Dialog::around(TextView::new("Confirm quit?"))
            .title("Quit P2P Scope?")
            .button("Cancel", |s| {
                s.pop_layer();
            }) //TOTO:  message ATTENTION:I QUIT to monolith chat and shut down libp2p
            .button("Confirm Quit", |s| {
                s.quit();
            }),
    );
}

// cb_sink send callbacks

fn append_to_tui_view(view_name: &str, from_id: String, message: Vec<u8>) {
    match view_name {
        "monolith_chat_view" => {
            cb_sink
                .send(Box::new(|s| {
                    s.call_on_name("monolith_chat_view", |view: &mut TextView| {
                        view.append(format!("From {}: {}\r", from_id, String::from_utf8_lossy(&message)));
                    })
                }))
                .unwrap();
        }
        "output_view" => {
            cb_sink
                .send(Box::new(|s| {
                    s.call_on_name("output_view", |view: &mut TextView| {
                        view.append(format!("{}\r", String::from_utf8_lossy(&message)));
                    })
                }))
                .unwrap();
        }
        _ => {
            cb_sink
                .send(Box::new(|s| {
                    s.call_on_name("output_view", |view: &mut TextView| {
                        view.append(format!(
                            "Unknown view\"{}\" message: \"{}\"\r",
                            view_name,
                            String::from_utf8_lossy(&message)
                        ));
                    })
                }))
                .unwrap();
        }
    }
}

// Argument parsing initialization
#[derive(Parser, Default, Debug)]
#[clap(author = "John Hall et. al.", version, about)]
struct Arguments {
    #[arg(long, value_enum)]
    listen_mode: Option<ListenMode>,
    #[arg(long)]
    dial: Option<Vec<Multiaddr>>,
    listen_on: Option<Multiaddr>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ListenMode {
    All,
    Localhost,
    Lan,
    NoListen,
}

fn terminal_output(output: S)
where
    S: Into<String>,
{
    append_to_tui_view("output_view", String::from(""), output.Into());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // env_logger::init();
    //parse command line arguments

    let clap_args = Arguments::parse();
    let args_text = format!("cli args: {:?}", clap_args);

    // INIT libp2p
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    let peer_id_text = format!("Instance peer id: {:?}", peer_id);

    // Terminal interface start
    let instance_info_text = format!(" {} CliArgs: {}", peer_id_text, args_text);
    let (user_message_sender, mut user_message_receiver) =
        tokio::sync::mpsc::channel::<Box<String>>(32);
    cursive::logger::init();
    let (cb_sink_sender, cb_sink_receiver) = oneshot::channel();
    let tui_handle = std::thread::spawn(move || {
        terminal_user_interface(user_message_sender, cb_sink_sender, peer_id_text, args_text);
    });

    let tus = tui_update_sender.clone();

    // More libp2p setup for NETWORK

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
    // NOW get the cb_sink And BLOCK so that terminal output is available
    let Ok(cb_sink) = cb_sink_receiver.blocking_recv();

    // Reach out to another node if specified
    match clap_args.dial {
        Some(addr_list) => {
            for addr in addr_list {
                swarm.dial(addr.clone());
                terminal_output(format!("Dialed {:?}", addr));
            }
        }
        None => {
            terminal_output(format!("No addresses Dialed"));
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
                terminal_output(format!("Not listening! La La la La La!"));
                no_listen = true;
            }
            ListenMode::Localhost => {
                swarm.listen_on(local_port)?;
            }
            ListenMode::Lan => {
                swarm.listen_on(all_ports)?;
                terminal_output(format!("LAN limitation unimplemented"));
            }
        }
    }
    let listeners = swarm.listeners();
    terminal_output("LISTENERS:\r".to_string());
    for ma in listeners {
        terminal_output(format!("{:?}\r", ma));
    }

    if let Some(maddr) = clap_args.listen_on {
        if !no_listen {
            swarm.listen_on(maddr)?;
        }
    }

    // Kick it off
    loop {
        tokio::select! {
            Some(box_message) = user_message_receiver.recv() => {
                let message = *box_message;
                swarm.behaviour_mut().floodsub.publish_any(
                    floodsub_topic.clone(), message);
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        terminal_output(     format!("Listening on {address:?}"));
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(
                        FloodsubEvent::Message(message))) => {
                        //unbounded std mpsc is not blocking
                        let message_string = format!("{}",String::from_utf8_lossy(&message.data));
                        tui_update_sender.send( Box::new(
                            TuiUpdate::AppendMessage(
                                "monolith".to_string(),
                                message.source.to_string() ,
                                message_string)));
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
                        terminal_output(     format!("Connected!: '{:?}'",event));
                        swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    }
                    SwarmEvent::ConnectionClosed {peer_id, endpoint: ConnectedPoint::Dialer { address,.. },
                        cause: Some(KeepAliveTimeout),..} => {
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        // Hanging up so rude! Redial !
                        // maybe a goodbye message. I believe this will only retry once.
                        terminal_output(     format!("KeepAliveTimeout, Redialing {:?}",address));
                        swarm.dial(address)?;
                    }
                    SwarmEvent::ConnectionClosed {peer_id,..} =>{
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        terminal_output(     format!("DRAT!:{:?}", event));
                    }
                    other_swarm_event => {
                        terminal_output(     format!("EVENT: '{:?}'",other_swarm_event));
                    }
                }
            }
        }
    }
}

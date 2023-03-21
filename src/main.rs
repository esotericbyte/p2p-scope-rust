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
use libp2p::{
    tcp, Multiaddr, PeerId, Transport, identity, mdns, mplex, noise,
    core::{upgrade},
    floodsub::{self, Floodsub, FloodsubEvent},
    futures::{StreamExt},
    swarm::{NetworkBehaviour, Swarm, SwarmEvent},
};
use libp2p::swarm::ConnectionError::KeepAliveTimeout;
use libp2p::core::{Endpoint, ConnectedPoint};

use std::error::Error;
use std::sync::mpsc;
use std::sync::mpsc::SendError;
use tokio::io::{AsyncBufReadExt};
use tokio;

// TUI and argument parsing includes
use clap::Parser;
use cursive;
use cursive::Cursive;
use cursive::reexports::toml::to_string;
use cursive::event::Event::Refresh;
use cursive::views::{Dialog, EditView, TextView, ListView, LinearLayout, ResizedView, ScrollView};
use cursive::view::{Nameable, Scrollable};

#[derive(Debug)]
enum TuiUpdate {
    // topic , from_id , message
    AppendMessage(String, String, String),
    NewContent(String, String),
}

struct TheCursiveUserData {
    user_message_sender: tokio::sync::mpsc::Sender<Box<String>>,
    tui_update_receiver: std::sync::mpsc::Receiver<Box<TuiUpdate>>,
}

fn terminal_user_interface(
    user_message_sender: tokio::sync::mpsc::Sender<Box<String>>,
    tui_update_receiver: std::sync::mpsc::Receiver<Box<TuiUpdate>>,
    instance_info_text: String,
) {

    // Initialize Cursive TUI
    let mut siv = cursive::default();
    //dark color scheme
    siv.load_toml(include_str!(
        "/home/johnh/rustsb/p2p-scope/p2p-scope-rust/src/colors.toml"
    )).unwrap();
    siv.set_user_data(TheCursiveUserData {
        user_message_sender,
        tui_update_receiver,
    });
    let e_to_do = cursive::event::Event::Ctrl();
    siv.add_global_callback(Refresh, move |s: &mut Cursive| {
        let ud: &TheCursiveUserData = s.user_data().unwrap();
        if let Ok(tui_update_boxed) = ud.tui_update_receiver.try_recv() {
            let tui_update = *tui_update_boxed;
            if let TuiUpdate::AppendMessage(topic, from_id, message) = tui_update {
                if topic == "monolith".to_string() {
                    s.call_on_name("monolith_chat_view", |view: &mut ListView| {
                        view.add_child("", TextView::new(format!("From {}: {}", from_id, message)));
                    });
                }
                if topic == "output".to_string() {
                    s.call_on_name("output_view", |view: &mut ListView| {
                        view.add_child("", TextView::new(message.to_string()));
                    });
                }
            }
        }
        cursive::event::EventResultEventResult::Ignore;
    });

    // CURSIVE  TUI views
    //let peers_view
    //let ports_view
    let user_message_input = EditView::new()
        .on_submit(new_user_message)
        .with_name("user_message_input");
    // let user_message_history = ListView::new()
    //    .with_name("user_message_history")
    //    .on_select(selected_message);
    let monolith_chat_view = ListView::new()
        .scrollable()
        .with_name("monolith_chat_view");
    let output_view = ListView::new()
        .scrollable()
        .with_name("output_view");
    let instance_info_view = TextView::new(instance_info_text);
    // let peers_and_ports_layout = LinearLayout::horizontal.new()
    //    .child(peers_view)
    //    .child(ResizedView.with_percent_width(20).child(ports_view))
    let scope_screen = ResizedView::with_full_screen(
        LinearLayout::vertical()
            .child(instance_info_view)
            //.child(peers_and_ports)
            .child(user_message_input)
            //.child(user_message_history)
            .child(monolith_chat_view)
            .child(output_view)
    );
    siv.add_layer(scope_screen);
    siv.run();
}

fn new_user_message(s: &mut Cursive, message: &str) {
    if message.is_empty() {
        return;
    };
    let msg = message.clone();
    s.call_on_name("monolith_chat_view", |view: &mut ListView| {
        view.add_child("", TextView::new(message))
    });
    //clear the user message view

    s.call_on_name("user_message_input", |view: &mut ListView| {
        view.clear();
    });
    let ud: &TheCursiveUserData = s.user_data().unwrap();
    ud.user_message_sender.blocking_send(Box::new(message.to_string()));
    //add to history
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
    No_listen,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
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
    let (tui_update_sender, tui_update_receiver) = std::sync::mpsc::channel();
    let (user_message_sender, mut user_message_receiver) = tokio::sync::mpsc::channel::<Box<String>>(32);
    let tui_handle = std::thread::spawn(move || {
        terminal_user_interface(user_message_sender, tui_update_receiver, instance_info_text);
    });
    fn tui_out_mk(tus: std::sync::mpsc::Sender<Box<TuiUpdate>>) -> Box<dyn Fn(String) -> Result<(), SendError<Box<TuiUpdate>>>> {
        Box::new(move |output: String| {
            tus.send(Box::new(TuiUpdate::AppendMessage(
                "output_view".to_string(), "".to_string(), output.to_string())))
        })
    }
    let tus = tui_update_sender.clone();
    let tui_out = tui_out_mk(tus);

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
    if let Some(addr_list) = clap_args.dial {
        for addr in addr_list {
            swarm.dial(addr.clone())?;
            tui_out(format!("Dialed {:?}",addr));
        }
    }
    // Replaced by Tui
    // Read full lines from stdin
    // let mut stdin = io::BufReader::new(io::stdin()).lines();

    // todo: mechanism to send stdout to Tui view
    let all_ports = "/ip4/0.0.0.0/tcp/0".parse()?;
    let local_port = "/ip4/127.0.0.0/tcp/0".parse()?;
    let mut no_listen = false;
    if let Some(ListenMode) = clap_args.listen_mode {
        match ListenMode {
            // Listen on all interfaces and whatever port the OS assigns
            ListenMode::All => { swarm.listen_on(all_ports)?; }
            ListenMode::No_listen => {
                tui_out(format!("Not listening! La La la La La!"));
                no_listen = true;
            }
            ListenMode::Localhost => { swarm.listen_on(local_port)?; }
            ListenMode::Lan => {
                swarm.listen_on(all_ports)?;
                tui_out(format!("LAN limitation unimplemented"));
            }
        }
    }
    if let Some(Maddr) = clap_args.listen_on {
        if !no_listen {
            swarm.listen_on(Maddr)?;
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
                        tui_out( format!("Listening on {address:?}"));
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
                        tui_out( format!("Connected!: '{:?}'",event));
                        swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    }
                    SwarmEvent::ConnectionClosed {peer_id, endpoint: ConnectedPoint::Dialer { address,.. },
                        cause: Some(KeepAliveTimeout),..} => {
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        // Hanging up so rude! Redial !
                        // maybe a goodbye message. I believe this will only retry once.
                        tui_out( format!("KeepAliveTimeout, Redialing {:?}",address));
                        swarm.dial(address)?;
                    }
                    SwarmEvent::ConnectionClosed {peer_id,..} =>{
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        tui_out( format!("DRAT!:{:?}", event));
                    }
                    other_swarm_event => {
                        tui_out( format!("EVENT: '{:?}'",other_swarm_event));
                    }
                }
            }
        }
    }
}

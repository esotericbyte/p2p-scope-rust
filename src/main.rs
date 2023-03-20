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

use mpsc::{channel,Sender,Receiver};
// Lib p2p and related includes
use futures::StreamExt;
use libp2p::{
    core::{upgrade},
    floodsub::{self, Floodsub, FloodsubEvent},
    identity, mdns, mplex, noise,
    swarm::{NetworkBehaviour, SwarmEvent },
    tcp, PeerId, Transport,
};
use libp2p_swarm::derive_prelude::ConnectedPoint::Dialer;
use std::error::Error;
use std::sync::mpsc;
use tokio::io::{self, AsyncBufReadExt};

// TUI and argument parsing includes
use clap::Parser;
use libp2p::Multiaddr;

use cursive::Cursive;
use cursive::views::*;
use cursive::theme;
use cursive::align::Align;
use cursive::event::Event::Refresh;
use cursive::traits::*;

#[derive(Debug)]
enum TuiUpdate {
    ChatMessage {topic : String, from_id : String, message : String},
    NewContent {view_name : String, content : String},
}

fn terminal_user_interface(
    user_message_sender: Sender<Box<String>>,
    tui_update_receiver: Receiver<Box<TuiUpdate>>,
    instance_info_text: String, 
    ) -> u8 {
    // Initialize Cursive TUI
    let mut siv = cursive::default();
    //dark color scheme
    siv.load_toml(
        include_str!(
            "/home/johnh/rustsb/p2p-scope/p2p-scope-rust/src/colors.toml")
        ).unwrap();
    siv.add_global_callback(
        Refresh,
        |s: & mut Cursive| {
        let tui_update = tui_update_receiver.try_recieve();
        match tui_update {
            Some(chat_message("monolith", from_id, message)) => {
                s.call_on_name("monolith_chat_view", |view: &mut ListView | {
                    view.add_child("",
                        TextView(format!("From {}: {}", from_id, message)));
                })
            }
            _ => {}
        }
    });
    
    // CURSIVE  TUI view setup
    //let peers_view
    //let ports_view
    let user_message_input = EditView::new()
        .with_name("user_message_input")
        .set_max_content_width(120)
        .set_filler(" ")            
        .on_submit(new_user_message);
    // let user_message_history = ListView::new()
    //    .with_name("user_message_history")
    //    .on_select(selected_message);
    let monolith_chat_view = Panel::new(())
        .title("monolith")
        .scrollable()
        .child(ListView::new()
        .with_name("monolith_chat_view"));
    let instance_info_view=TextView(instance_info_text);
    // let peers_and_ports_layout = LinearLayout::horizontal.new()
    //    .child(peers_view)
    //    .child(ResizedView.with_percent_width(20).child(ports_view))  
    let scope_screen =
        ResizedView::with_full_screen(LinearLayout::vertical()
           .child(instance_info_view)
            //.child(peers_and_ports)
           .child(user_message_input)
            //.child(user_message_history)
           .child(monolith_chat_view)
            //.child(events)
        );
    siv.add_layer(scope_screen);
    siv.run();
}

fn new_user_message(s: &mut Cursive, message: &str){
    if mesage.is_empty(){ return };
    let msg = message.clone();
    s.call_on_name(
        "monolith_chat_view",
        |view:&mut ListView| {
            view.add_child("",TextView(message))
        }
    );
    user_message_sender.send(message)
    //add to history
    //clear the user message view
}

// fn gen_handle_ui_update(tui_update_receiver: mpsc::Receiver<Box<TuiUpdate>>) ->
//     Box<dyn Fn(& mut cursive)> {
//     Box::new(
//     |s: & mut cursive| {
//         tui_update = tui_update_receiver.try_recieve();
//         match ui_update {
//             Some(ChatMessage("monolith", from_id, message)) => {
//                 s.call_on_name("monolith_chat_view", |view: &mute ListView | {
//                     view.add_child(
//                         TextView::new(format!("From {}: {}", from_id, message)));
//                 })
//             }
//             _ => {}
//         }
//     })
// }



fn dlg_on_quit(s: &mut Cursive){
    s.add_layer(Dialog::around(TextView::new("Confirm quit?"))
        .title("Quit P2P Scope?")
        .button("Cancel", |s| {
            s.pop_layer();
        }) //TOTO: Insert an ATENTION:I QUIT message to monolith chat and shut down libp2p
        .button("Confirm Quit", |s| {
            s.quit();
        })
    );
}


// Argument parsing initialization
#[derive(Parser,Default,Debug)]
#[clap(author="John Hall et. al.", version, about)]
struct Arguments {
    #[arg(long,value_enum)]
    listen: Option<ListenMode>,
    #[arg(long)]
    dial: Option<Vec<Multiaddr>>
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ListenMode {Deaf, All, Localhost, Lan, Choose}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    //parse comand line arguements
    let args = Arguments::parse();
    let args_text = format!("cli args: {:?}",args);
 
    // Terminal interface start
    let instance_info_text = format!("Instance Info: {} {}",instance_id_text, args_text);
    let (tui_update_sender,tui_update_receiver) = channel(); // for TuiUpdate 
    let (user_message_sender, user_message_receiver) = channel(); //for String
    std::thread::spawn(move|| {
        terminal_user_interface(user_message_sender,
        tui_update_receiver,
        instance_info_text);
    });

    // INIT libp2p
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    let peer_id_text = format!("Instance peer id: {peer_id:?}");

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
    let mut swarm = libp2p_swarm::Swarm::with_tokio_executor(transport, behaviour, peer_id);
    swarm.behaviour_mut().floodsub.subscribe(floodsub_topic.clone());

    // Reach out to another node if specified
    if let Some(to_dial) = std::env::args().nth(1) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("Dialed {to_dial:?}");
    }

    // Read full lines from stdin
    //let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Kick it off
    loop {
        tokio::select! {
            Some(user_message) = user_message_receiver.try_recv() => {
                swarm.behaviour_mut().floodsub.publish_any(
                    floodsub_topic.clone(), line_str.as_bytes());
            }
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {address:?}");
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Floodsub(FloodsubEvent::Message(message))) => {
                        println!(
                                "Received: '{:?}' from {:?}",
                                String::from_utf8_lossy(&message.data),
                                message.source
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
                        println!("Connected!: '{:?}'",event);
                        swarm.behaviour_mut().floodsub.add_node_to_partial_view(peer_id);
                    }
                    SwarmEvent::ConnectionClosed {peer_id, endpoint: Dialer { address,.. },
                        cause: Some(libp2p_swarm::ConnectionError::KeepAliveTimeout),..} => {
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        // Hanging up so rude! Redial !
                        // maybe a goodbye message. I believe this will only retry once.
                        println!("KeepAliveTimeout, Redialing {:?}",address);
                        swarm.dial(address)?;
                    }
                    SwarmEvent::ConnectionClosed {peer_id,..} =>{
                        swarm.behaviour_mut().floodsub.remove_node_from_partial_view(&peer_id);
                        println!("DRAT!:{:?}", event)
                    }
                    other_swarm_event => {
                        println!("EVENT: '{:?}'",other_swarm_event);
                    }
                }
            }
        }
    }
}




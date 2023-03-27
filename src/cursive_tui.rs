
// Cursive TUI api
use cursive;
use cursive::{backend, backends, CbSink, crossterm, Cursive};
use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::theme::*;
use cursive::traits::*;
use cursive::view::{Nameable, Position, Scrollable};
use cursive::views::{
    Button, Dialog, EditView, LinearLayout, Panel, ResizedView, ScrollView, TextView,
};

use std::sync::mpsc;
// fully specify tokio::sync::mpsc

use std::sync::Mutex;
use std::sync::Arc;
use cursive::backends::puppet::Backend;
use crate::{CursiveChannelRunner, Multiaddr};
use crate::PeerId;
use crate::Arguments;
use crate::cursive_channel_runner::CursiveChannelRunner;

#[derive(Debug)]
pub(crate) enum TuiUpdate {
    // Todo: Add Times
    // topic , from_id , message
    TextMessage(ViewSpec, tup::PeerID(), tup::MessageText()),
    InputMessage(tup::Topic, tup::MessageText()),
    ProtobufMessage(tup::Topic, tup::PeerID(),tup::SpecProtobuf(), tup::MessageProtobuf()),
    // arbitrary program output to output_view
    TerminalOutput(tup::MessageText()),
    AppendView(ViewSpec, String),
    NewContent(ViewSpec, String),
    // NewEvent(time,source,environment,event)
}
// TuiUpdateParts = tup
#[derive(Debug)]
enum tup{
    Topic(String),
    MessageText(String),
    MessageProtobuf(Vec<u8>),
    SpecProtobuf(String),
    PeerID(PeerId),
}

#[derive(Debug)]
enum ViewSpec {
    ViewName(String),
    ViewIdS(String),
    ViewIdI(i32)
}

// cursive allows to store a user data in it's runtime so this struct is for that purpose
#[derive(Debug)]
struct TheApiUserData {
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    libp2p_network_id: String,
    command_line_opts: Box<Arguments>,
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
pub fn terminal_user_interface(
    // Topic , Message
    input_sender: tokio::sync::mpsc::Sender<Box<TuiUpdate>>,
    libp2p_network_id: String,
    command_line_opts: Box<Arguments>,
)
{
    // Initialize Cursive TUI
    cursive::logger::init();

    let mut curs = CursiveChannelRunner::new(
        cursive::Cursive::new(),
        backends::crossterm,
        update_receiver);

    //todo:light color scheme
    //dark color scheme
    curs.load_toml(include_str!("colors.toml")).unwrap();
    let user_data = TheApiUserData {
        input_sender,
        libp2p_network_id: libp2p_network_id.clone(),
        command_line_opts: command_line_opts.clone(),
    };
    curs.set_user_data(user_data); //value as &dyn Any
    curs.add_global_callback(
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
        .min_width(35)
        .min_height(7)
        .scrollable();
    let output_view = TextView::new("OUTPUT VIEW\r")
        .with_name("output_view")
        .min_width(35)
        .min_height(3)
        .max_height(10)
        .scrollable();

    let instance_info_view = TextView::new(
        format!("Peer ID: {} Command Arguments: {}", libp2p_network_id, command_line_opts))
        .with_name("instance_info")
        .full_width()
        .min_height(2);

    // let peers_and_ports_layout = LinearLayout::horizontal.new()
    //    .child(peers_view)
    //    .child(ResizedView.with_percent_width(20).child(ports_view))
    // todo: add a menu? or commands? or both? Commands are better because then it's scriptable
    // todo: create a better layout. make a reactive and proportional option
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
    curs.add_layer(scope_screen);
    curs.run();
}
// inital callbacks for declaritive phase
fn new_user_message(s: &mut Cursive, message: &str) {
    s.call_on_name("monolith_chat_view", |v: &mut TextView| {
        v.append(format!("{}\r", message))
    });
    s.call_on_name("user_message_input", |v: &mut EditView| v.set_content(""));
    let ud: &TheApiUserData = s.user_data().unwrap();
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

//  cb_sink send callbacks
//
// fn append_to_tui_view(cb_sink: Box<&CbSink>,view_name: &str, from_id_c: String, message_c: String) {
//     let from_id = from_id_c.clone();
//     let message = message_c.clone();
//     match view_name {
//         "monolith_chat_view" => {
//             cb_sink.send(Box::new( move |s| {
//                 s.call_on_name("monolith_chat_view", |view: &mut TextView| {
//                     view.append(format!("From {}: {}\r", from_id, message));
//                 }); })).unwrap();
//         }
//         "output_view" => {
//             cb_sink.send (Box::new(move |s| {
//                 s.call_on_name("output_view", |view: &mut TextView| {
//                     view.append(format!("{}\r", message));
//                 });
//             })).unwrap();
//         }
//         _ => {
//             let out_message =
//                 format!("Unknown view\"{}\" message: \"{}\"\r", String::from(view_name), message);
//
//             cb_sink.send(Box::new( move |s| {
//                 s.call_on_name("output_view", |view: &mut TextView| {
//                     view.append(out_message);
//                 });
//             })).unwrap();
//         }
//     }
// }

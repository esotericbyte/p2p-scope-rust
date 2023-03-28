
// Cursive TUI api
use cursive;
use cursive::{Callback, CbSink, crossterm, Cursive, CursiveRunnable};
use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::traits::*;
use cursive::view::{Nameable, Position, Scrollable};
use cursive::views::{
    Button, Dialog, EditView, LinearLayout, Panel, ResizedView, ScrollView, TextView,
};

use std::sync::mpsc;
// fully specify tokio::sync::mpsc
use std::sync::Mutex;
use std::sync::Arc;
use cursive::event::Callback;
use crate::{Multiaddr, PeerId, CliArguments, Theme};

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
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    lib_p2p_network_id: PeerId,
    command_line_opts: CliArguments,
    mut curs : CursiveRunnable,
)
{
    // Initialize Cursive TUI
    cursive::logger::init();

    //dark color scheme
    if command_line_opts.theme != Some(Theme::Light) {
        curs.load_toml(include_str!("colors.toml")).unwrap();
    }

    curs.set_user_data( TheApiUserData {
        input_sender,
        lib_p2p_network_id,
        command_line_opts,
    });

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
        format!("Peer ID: {} Command Arguments: {:?}", lib_p2p_network_id, command_line_opts))
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
// callbacks to customize the ui during declarative cursive phase to prepare the running phase
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


pub fn ui_update_to_cursive_callback(ui_update: UiUpdate) -> Callback {
        match ui_update {
            ui_update::TextMessage(Tup::Topic("monolith"),Tup(peer_id),Tup(message)) => {
                Box::new(move |s : &mut Cursive| {
                    s.call_on_name("monolith_chat_view", |view: &mut TextView| {
                        view.append(format!("ⅈ{:?}ⅈSENT\r    {}\r", peer_id, message));})
                        .unwrap();
                    }).unwrap()
            }
            ui_update::TerminalOutput(Tup::MessageText(message)) =>{
                Box::new(move |s : &mut Cursive| {
                 s.call_on_name("output_view",
                                |view: &mut TextView| { view.append(format!("{}\r", message));})
                     .unwrap()
                 }).unwrap()
            }
            _ => {
                let out_message =format!("Update Unimplemented: \"{:?}\"\r", ui_update);
                Box::new(move |s : &mut Cursive| {
                 s.call_on_name("output_view",
                                |view: &mut TextView| { view.append(out_message); })
                     .unwrap();
                }).unwrap()
            }
    }
}

#[derive(Debug)]
pub(crate) enum UiUpdate {
    // Todo: Add Times for events
    // Many of these are preliminary
    // The purpose is to create types that are independent of UI implementation
    TextMessage(Tup::Topic(string), Tup::PeerID(), Tup::MessageText()),
    InputMessage(Tup::Topic, Tup::MessageText()),
    ProtobufMessage(Tup::Topic, Tup::PeerID(), Tup::SpecProtobuf(), Tup::MessageProtobuf()),
    // arbitrary program output to output_view
    TerminalOutput(Tup::MessageText()),
    // Prehaps too Cursive implementation specific
    AppendToView(ViewSpec, String),
    NewViewContent(ViewSpec, String),
    // NewEvent(time,source,environment,event)
}
//the UiUpdatePart = Tup
#[derive(Debug)]
pub(crate) enum Tup {
    Topic(String),
    MessageText(String),
    MessageProtobuf(Vec<u8>),
    SpecProtobuf(String),
    PeerID(PeerId),
}

#[derive(Debug)]
pub(crate) enum ViewSpec {
    ViewName(String),
    ViewIdS(String),
    ViewIdI(i32)
}

// cursive allows to store a user data in it's runtime so this struct is for maximizing that.
#[derive(Debug)]
pub(crate) struct TheApiUserData {
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    lib_p2p_network_id: PeerID,
    command_line_opts: CliArguments,
}

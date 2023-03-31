
// Cursive TUI api
use cursive;
use cursive::{ CbSink, crossterm, Cursive, CursiveRunnable};
use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::traits::*;
pub use cursive::view::{Nameable, Position, Scrollable};
pub use cursive::views::{
    Button, Dialog, EditView, LinearLayout, Panel, ResizedView, ScrollView, TextView,
};
pub type CursiveCallback = dyn FnOnce(&mut Cursive) + Send;
// fully specify tokio::sync::mpsc
use libp2p::{Multiaddr, PeerId};
use crate::{CliArguments, Theme};

// Cursive  UI has 2 phases
// In the first phase the UI is declared
// In the second phase it is run on an event loop in a standard synchronous thread.
// See  "More about Cursive.md for more notes on this UI implementation"

pub fn terminal_user_interface(
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    lib_p2p_network_id: PeerId,
    command_line_opts: CliArguments,
    cb_sync_sender: tokio::sync::oneshot::Sender<CbSink>
)
{
    let mut curs = cursive::default();
    cb_sync = curs.cb_sink();
    cb_sync_sender.send(cb_sync);

    // Initialize Cursive TUI
    cursive::logger::init();

    //dark color scheme
    match command_line_opts.theme {
        Some(Theme::Light) => {
            (); //For now use defaults for light theme
        }
        Some(Theme::Dark) => {
            curs.load_toml(include_str!("colors.toml")).unwrap();
        }
        None => {
            curs.load_toml(include_str!("colors.toml")).unwrap();
        }
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
    ud.input_sender
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

pub fn ui_update_to_cursive_callback(ui_update: UiUpdate) -> Box<CursiveCallback> {
        match ui_update {
            UiUpdate::TextMessage(topic , peer_id , message) => {
                if topic == String::from("monolith") {
                    Box::new(move |s: &mut Cursive| {
                        s.call_on_name("monolith_chat_view", |view: &mut TextView| {
                            view.append(format!("ⅈ{:?}ⅈSENT\r    {}\r", peer_id, message));
                        })
                            .unwrap()
                    })
                } else {
                        let out_message =format!("Update Unimplemented: \"{:?}\"\r", ui_update);
                        Box::new(move |s : &mut Cursive| {
                            s.call_on_name("output_view",
                                           |view: &mut TextView| { view.append(out_message); })
                                .unwrap()
                        })
                    }
            }
            UiUpdate::TerminalOutput(message) =>{
                Box::new(move |s : &mut Cursive| {
                    s.call_on_name("output_view",
                                   |view: &mut TextView|
                                       { view.append(format!("{}\r", message)); })
                        .unwrap()
                })
            }
            _ => {
                let out_message =format!("Update Unimplemented: \"{:?}\"\r", ui_update);
                Box::new(move |s : &mut Cursive| {
                 s.call_on_name("output_view",
                                |view: &mut TextView| { view.append(out_message); })
                     .unwrap()
                })
            }
    }
}
//Implementation independent UI message types
#[derive(Debug)]
pub(crate) enum UiUpdate {
    // Todo: Add Times for events and times between them
    // NewEvent(time,source,event,environment,related)
    TextMessage(String,PeerId,String),//Topic, PeerID, Message
    InputMessage(String),// MessageText
    // arbitrary program output to output_view
    TerminalOutput(String),
    AppendToView(ViewSpec, String),
    ReplaceViewContent(ViewSpec, String),

}
//the UiUpdatePart = Tup
// This idea of string flags might be done instead by creating an impl
// for an emum with constructors the values in the tuple wih it's function
// parameter names.
// It isn't possible to use a partial value (enum variant) for a Type name
// It's no longer very concise to use Tup type in UiUpdate and then specify
// or only match the constraints.
// #[derive(Debug)]
// pub(crate) enum Tup {
//     Topic(String),
//     MessageText(String),
//     MessageProtobuf(Vec<u8>),
//     SpecProtobuf(String),
//     PeerID(PeerId),
// }

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
    lib_p2p_network_id: PeerId,
    command_line_opts: CliArguments,
}

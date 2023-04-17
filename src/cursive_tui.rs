use std::any::type_name;
// Cursive TUI api
use cursive;
use cursive::direction::Orientation::{Horizontal, Vertical};
use cursive::traits::*;
pub use cursive::view::{Nameable, Position, Scrollable};
pub use cursive::views::{
    Button, Dialog, EditView, LinearLayout, Panel, ResizedView, ScrollView, TextView,
};
use cursive::{crossterm, CbSink, Cursive, CursiveRunnable, align};
use cursive::utils::span::SpannedString;
use cursive::reexports::enumset::EnumSet;
use cursive::theme::*;
use cursive::theme::Style;

pub type CursiveCallback = dyn FnOnce(&mut Cursive) + Send;
// fully specify tokio::sync::mpsc
use crate::{CliArguments, Theme};
use libp2p::{Multiaddr, PeerId};

// Cursive  UI has 2 phases
// In the first phase the UI is declared
// In the second phase it is run on an event loop in a standard synchronous thread.
// See  "More about Cursive.md for notes and considerations for p2p applications."

pub fn terminal_user_interface(
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    lib_p2p_network_id: PeerId,
    command_line_opts: CliArguments,
    cb_sync_sender: tokio::sync::oneshot::Sender<CbSink>,
) {
    let mut curs = cursive::default();
    let mut cb_sink = curs.cb_sink().clone();
    cb_sync_sender.send(cb_sink);

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

    curs.set_user_data(TheApiUserData {
        input_sender,
        lib_p2p_network_id,
        command_line_opts: command_line_opts.clone(),
    });

    curs.add_global_callback(
        cursive::event::Event::CtrlChar('d'),
        move |s: &mut Cursive| {
            s.toggle_debug_console();
        },
    );

    curs.add_global_callback(
        cursive::event::Event::CtrlChar('c'),
        dlg_on_quit,
    );

// Declare views to compose
    let user_message_input = EditView::new()
        .on_submit(new_user_message)
        .with_name("user_message_input")
        .min_width(40)
        .full_width();

    let monolith_chat_view = scope_data_panel(
        "monolith_chat_view",
        "Monolith Chat",
        "   Start of chat for this node ");

    let output_view = scope_data_panel(
            "output_view",
            "General Output",
            "Output Start.");



    let instance_info_view =
        TextView::new(format!("Peer ID: {} Command Arguments: {:?}",
        lib_p2p_network_id, command_line_opts))
        .with_name("instance_info")
        .full_width()
        .min_height(2);

   // App info window view with versions of Libs and Rust
   // Construction kit for network behavior

    // Network message data template for json cbor bson style messages given a schema
    // Json schema , XML , Protobuff, Avero

    // let user_input_history = TextView::new()
    //    .with_name("user_message_history")
    //    .on_select(selected_message);

    // CURSIVE  TUI views
    //let peers_view
    //let ports_view

    // Some settings or code-derivative file that allows a view into
    // The Transport
    // The Behaivor
    // That is no specific to chat-tokio

    // ?? Develop views to show protocol messages and data packets??
    // Probably best to use it together with a wire tool so no lower than
    // protocol if even that low level.

    // let peers_and_ports_layout = LinearLayout::horizontal.new()
    //    .child(peers_view)
    //    .child(ResizedView.with_percent_width(20).child(ports_view))

    // todo: add a menu? or commands? or both? Commands are better because then it's scriptable
    // todo: create a better layout. make a reactive and proportional option
    let scope_screen = ResizedView::with_full_screen(
        Panel::new(
        LinearLayout::vertical()
            .child(instance_info_view)
            //.child(peers_and_ports)
            .child(user_message_input)
            //.child(user_message_history)
            .child(
                LinearLayout::new(Horizontal)
                    .child(monolith_chat_view)
                    .child(output_view),
            )).title("P2P Scope - Alpha" )
    );
    curs.add_layer(scope_screen);
    curs.run();
}

// Declarative helper functions
fn scope_data_panel<S,Q>(name: S, title: S, start_text: Q)
    -> impl View
where Q: Into<SpannedString<Style>>, S: Into<String>  {
    Panel::new( TextView::new( start_text)
        .with_name(name)
        .min_width(20).min_height(7).scrollable())
        .title(title)
        .title_position(align::HAlign::Left)
}


// callbacks to customize the ui during declarative cursive phase to prepare the running phase
fn new_user_message(s: &mut Cursive, message: &str) {
    s.call_on_name("monolith_chat_view", |v: &mut TextView| {
        v.append(format!("{}\r", message))
    });
    s.call_on_name("user_message_input", |v: &mut EditView| v.set_content(""));
    let ud: &TheApiUserData = s.user_data().unwrap();
    ud.input_sender
        .blocking_send(Box::new(message.to_string())).unwrap();
    //TODO: add messages to history
    //TODO send as an event not boxed stringk
    //TODO internal command parsing
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
        UiUpdate::TextMessage(topic, peer_id, message) => {
            if topic == String::from("monolith") {
                Box::new(move |s: &mut Cursive| {
                    s.call_on_name("monolith_chat_view", |view: &mut TextView| {
                        view.append(format!("ⅈ{:?}ⅈSENT\r    {}\r", peer_id, message));
                    })
                    .unwrap()
                })
            } else {
                let out_message = format!(
                    "Update Unimplemented: ❝{:?}❞\r",
                    UiUpdate::TextMessage(topic, peer_id, message)
                );
                Box::new(move |s: &mut Cursive| {
                    s.call_on_name("output_view", |view: &mut TextView| {
                        view.append(out_message);
                    })
                    .unwrap()
                })
            }
        }
        UiUpdate::TerminalOutput(message) => Box::new(move |s: &mut Cursive| {
            s.call_on_name("output_view", |view: &mut TextView| {
                view.append(format!("{}\r", message));
            })
            .unwrap()
        }),
        _ => {
            let out_message = cursive::utils::markup::markdown::parse(
                format!("**Unimplemented!** ❝{:?}❞\r", ui_update));
            Box::new(move |s: &mut Cursive| {
                s.call_on_name("output_view", |view: &mut TextView| {
                    view.append(out_message);
                })
                .unwrap()
            })
        }
    }
}

// // align::HAlign::Left
// fn ScrollingTextDataPanel(name,title,start_text,mwidth usize,
// mheight usize,align_val){
//     Panel::new(
//         TextView::new("{} \r",teststart)
//             .with_name(name)
//             .min_width(mwidth )
//             .min_height(mheight)
//             .scrollable())
//         .set_title_position(align_val)
//         .set_title(title);
// }
//


//Implementation independent UI message types
#[derive(Debug)]
pub enum UiUpdate {
    // Todo: Add Times for events and times between them
    // NewEvent(time,source,event,environment,related)
    TextMessage(String, PeerId, String), //Topic, PeerID, Message
    InputMessage(String),                // MessageText
    // arbitrary program output to output_view
    TerminalOutput(String),
    AppendToView(ViewSpec, String),
    ReplaceViewContent(ViewSpec, String)
}

#[derive(Debug)]
pub enum ViewSpec {
    ViewName(String),
    ViewIdS(String),
    ViewIdI(i32),
}

// cursive allows to store a user data in it's runtime so this struct is for maximizing that.
#[derive(Debug)]
pub(crate) struct TheApiUserData {
    input_sender: tokio::sync::mpsc::Sender<Box<String>>,
    lib_p2p_network_id: PeerId,
    command_line_opts: CliArguments,
}


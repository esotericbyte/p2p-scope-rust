use clap::Parser;

#[derive(Parser,Default,Debug)]
#[clap(author="John Hall", version, about)]
/// Arrange and view libp2p events
struct Arguments {
    #[clap(short,long)]
    listen: Option<String>,
    dial: Option<String>
}

use cursive::Cursive;
use cursive::views::{ Button, Dialog, DummyView, EditView,
                     LinearLayout, TextView, SelectView,
                     ThemedView,Layer };
use cursive::theme;
use cursive::traits::*;
use cursive_table_view::{TableView,TableViewItem};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum PeerCol {Id, Role, maddr, live}

impl PeerCol {
    fn as_str(&self) -> &str{
        match *self{
            PeerCol::id => "ID",
            PeerCol::role => "Role",
            PeerCol::maddr => "Maddr",
            PeerCol::live => "live",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq,Hash)]
enum PeerRole{ Dialer, Listener, Relay}
impl PeerRole {
    fn as_str(&self) -> &str{
        match *self{
            PeerRole::Dialer => "D",
            PeerRole::Listenr => "L",
            PeerRole::Relay => "R",
        }
    }
}

#[derive(Clone, Debug)]
struct PeerRow {
    id: String,
    role: PeerRole,
    maddr: String,
    live: bool,
}

Impl TableViewItem<PeerCol> for PeerRow {
    fn to_column(&self, column: PeerCol) -> String {
        match column{
            PeerCol::id => self.id.to_string(),
            PeerCol::maddr => self.maddr.to_string(),
            PeerCol::role => self.role.to_string(),
            PeerCol::live => if self.live {"*"} else {" "},
        }
    }
}




#[derive(Copy, Clone, Hash, Debug)]
struct InstanceInfo{
    id: String,
    host: String,
    working_directory: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]

enum ListenerCol{
    maddr, connection_count
}
impl ListenerCol {
    fn as_str(&self) -> &str{
        match *self{
            ListenerCols::maddr => "Maddr",
            ListenerCols::connection_count => "ConCount",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct ListenerRow{
    maddr:String,
    connection_count: u16,
}
Impl TableViewItem<ListenerCol> for ListenerRows {
    fn to_column(&self, column: ListenerCol) -> String {
        match column{
            ListenerCol::maddr => self.maddr.to_string(),
            ListenerCol::connection_count => format!("{}",self.connection_count),
        }
    }
}

fn main() {
    let args = Arguments::parse();
    println!("{:?}",args);

    let mut siv = cursive::default();

    siv.load_toml(include_str!("/home/johnh/rustsb/p2p-scope/p2p-scope-rust/src/colors.toml")).unwrap();
 
    let select = SelectView::<String>::new()
        .on_submit(on_submit)
        .with_name("select")
        .fixed_size((10, 5));
    let peers_and_ports= LinearLayout::horizontal().
        .child(
    let buttons = LinearLayout::vertical()
        .child(Button::new("Add new", add_name))
        .child(Button::new("Delete", delete_name))
        .child(DummyView)
        .child(Button::new("Quit", dlg_on_quit));

    let theme = siv.current_theme().clone().with(|theme| {
        theme.palette[theme::PaletteColor::View] = theme::Color::Dark(theme::BaseColor::Black);
        theme.palette[theme::PaletteColor::Primary] = theme::Color::Light(theme::BaseColor::Green);
        theme.palette[theme::PaletteColor::TitlePrimary] =theme::Color::Light(theme::BaseColor::Green);
        theme.palette[theme::PaletteColor::Highlight] = theme::Color::Dark(theme::BaseColor::Green);
    });
    
    
    siv.add_layer(ThemedView::new(theme,
        Layer::new(Dialog::around(LinearLayout::horizontal()
            .child(select)
            .child(DummyView)
            .child(buttons))
        .title("Select a profile"))));

    siv.run();
}

fn add_name(s: &mut Cursive) {
    fn ok(s: &mut Cursive, name: &str) {
        s.call_on_name("select", |view: &mut SelectView<String>| {
            view.add_item_str(name)
        });
        s.pop_layer();
    }

    s.add_layer(Dialog::around(EditView::new()
            .on_submit(ok)
            .with_name("name")
            .fixed_width(10))
        .title("Enter a new name")
        .button("Ok", |s| {
            let name =
                s.call_on_name("name", |view: &mut EditView| {
                    view.get_content()
                }).unwrap();
            ok(s, &name);
        })
        .button("Cancel", |s| {
            s.pop_layer();
        }));
}

fn delete_name(s: &mut Cursive) {
    let mut select = s.find_name::<SelectView<String>>("select").unwrap();
    match select.selected_id() {
        None => s.add_layer(Dialog::info("No name to remove")),
        Some(focus) => {
            select.remove_item(focus);
        }
    }
}

fn dlg_on_quit(s: &mut Cursive){
    s.add_layer(Dialog::around(TextView::new("Confirm quit?"))
        .title("Quit P2P Scope?")
        .button("Cancel", |s| {
            s.pop_layer();
        })
        .button("Confirm Quit", |s| {
            s.quit();
        })
    );
}

fn on_submit(s: &mut Cursive, name: &str) {
    s.pop_layer();
    s.add_layer(Dialog::text(format!("Name: {}\nAwesome: yes", name))
        .title(format!("{}'s info", name))
        .button("Quit", Cursive::quit));
}


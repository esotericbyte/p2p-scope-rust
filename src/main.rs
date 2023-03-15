use clap::Parser;
use libp2p::Multiaddr;

use cursive::Cursive;
use cursive::views::*;
use cursive::theme;
use cursive::align::Align;
use cursive::traits::*;
use cursive_table_view::{TableView,TableViewItem};
//use ui_data.rs;

#[derive(Parser,Default,Debug)]
#[clap(author="John Hall", version, about)]
struct Arguments {
    #[arg(long,value_enum)]
    listen: Option<ListenMode>,
    #[arg(long)]
    dial: Option<Vec<Multiaddr>>
}
#[derive(clap::ValueEnum, Clone, Debug)]
enum ListenMode {All, Localhost, Lan, Choose, Deaf}


fn main() {
    let args = Arguments::parse();
    println!("{:?}",args);

    let mut siv = cursive::default();
    //dark color scheme
    siv.load_toml(
        include_str!(
            "/home/johnh/rustsb/p2p-scope/p2p-scope-rust/src/colors.toml")
        ).unwrap();
 
    let select = SelectView::<String>::new()
        .on_submit(on_submit)
        .with_name("select")
        .fixed_size((30, 12));

   // let peers_and_ports= LinearLayout::horizontal().
   //     .child(
    
    let buttons = LinearLayout::vertical()
        .child(Button::new("Add new", add_name))
        .child(Button::new("Delete", delete_name))
        .child(DummyView)
        .child(Button::new("Quit", dlg_on_quit));
   
    
    siv.add_layer(ResizedView::with_full_screen(Panel::new(LinearLayout::horizontal()
            .child(select)
            .child(DummyView)
            .child(buttons)
            )));

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
            .fixed_width(35))
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


//Table view data logic structs and traits
// As well as other interface data elements

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



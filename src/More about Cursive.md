Cursive terminal interface
Why?
I was working on an example that was not working. The output was mixed together and basically a mess. Was a command line sufficient for what was only going to get more complex? I decided hell no, but also did not want to go to any great lengths or get distracted with many details by adding something elaborate like a local web panel which would also complicate analyzing the p2p network traffic.  
Cursive runs in a thread and surrenders time to other apps and threads by scheduled sleeps in an event loop like most GUI apps. It uses standard functions not futures.  
Tokio runtime and libp2p is completely different from cursive because of the different problems they solve.

They are 2 separate event loops and will be run in separate threads. They send messages back and forth to deliver input and update the UI. 

The primary way to communicate into the Cursive thread is provided by the Crusive API. During the run phase a crossbeam mpsc chanel for closures and the cursive runtime checks and executes the closures on the channel regularly as part of the event loop.

Sending callbacks is a highly coupled way to implement messaging and I wanted to avoid any cursive specific code beyond minimal declarations in the main.rs code. The cursive code is in a cursive_tui.rs file and messages that are more implementation independent are translated to the Cursive tui callbacks. This is at the cost of a little bit more code to maintain but this makes the effort more reusable and consolidated for extending it and also for creating examples using other interface frameworks.   

To prepare for creating the thread for the UI make the tokio channel for sending input events and an empty new Cursive runner instance.  Get a  callback sender channel from the instance. Spawn the Cursive runtime and pass the tokio mpsc Sender and the cursive runner into terminal_user_interface.  Retain the cb_sink channel from cursive and the tokio mpsc receiver in the main thread. For the rest of the run of the application send manages back and forth.

I choose a full-screen UI that is not dialog driven like many cursive apps. I don't see that style as a good fit for this app because the goal is to display and navigate information about the switch / swarm events , peers, messages and resulting functioning as I develop a larger application.

Understanding Cursive 
P2P scope is built to be a good place to start and be able to view track and debug your libp2p project. I hope to add some modules to it to extend it for all and hope that others find ways to extend and improve it. To that end I'd like to share my experience with Cursive. I believe it's a worthy direction for creating a simple maintainable portable UI you can customize for your own needs.

Cursive uses a kind of global object for it's tread that is the root of a data tree of views and a type container for methods to manage the tree and the views. To create a complex application use a declarative style to define view types and their layout. Many examples use a peculiar retro style of terminal dialogs, but it is completely possible to use more modern styling and full-screen layouts. 

There is an unusual object model going on in Cursive. A "view" is it's own kind of object model borrowing from a widget or component but also, a view can serve to simply add properties and functionality to an existing view by wrapping it, and the order of these wrappers matters. This is quirky enough to say that Cursive has a significant learning curve.

At the cursive root you by default have menu bar at the top and a separate API for interacting with this menu. Cursive manages a stack of screens each of which is a tree root. Only the top of this stack is active. After declaring a view for a screen you can define its layout by adding children or adding properties by wrapping a 'view' in another type of 'view' with a property. These include a size constraint, name, id, scrolling, and event subscriptions. 

It is a zany way to manage things and I have found this tiresome to a degree but then realized that the code does not have to deal with expectations that every 'view' has properties few actually use. The tricky part is that in some cases warping order matters. This isn't really documented so follow examples or look forward to some trial and error.  

Callbacks in the form of closures are the primary way to send changes to the runtime.
Cursive has a 'user data' instance within the
root object. This is used to store a tokio chanel to send data out of the ui thread to the tokio runtime.

Cursive can be changed during runtime as well as exchanged for a different layout,serialized, persisted and restored so layout is not fixed at runtime. After understanding these features of cursive and learning some quirks it's easy to imagine an app that uses this cross-platform terminal interaction api for dynamic components to share various interactions over your new peer to peer networks.

I plan to start with cursive but create systems that use web technologies as a primary facade. It is because of the sufficiency of cursive and direct use of local hardware it's an excellent choice for a back end view, and a first shot at a usable interface to create function and define needs before getting into design. It's about storyboarding and prototyping.

See UI alternatives.md for further discussion. 

This is a project for getting started with a project that uses libp2p.
This project extends the chat example from rust-libp2p with a terminal user interface. 
Instaed of mdns this version uses coomand line options and does not use MDNS.
To start the swarm run the program and note it's listening address Multiaddr. 
Lanch a second terminal or another pc on the same network and use the --dial option with the listening address from the first or subsequent peer.

It has a terminal user interface that runs along side the async networking event loop. It has two output areas by default. One for the "monotlith" chat and the other for information and debug messages.
If you don't see both, resize your terminal window.   
I plan to add options and commands, tracing support and the ability to configure and test libp2p swarms with the ability to introspect into the running application.

I plan to modularize the code so that the libp2p networking and event loop can be easily isolated and used in other projects.
The current version uses floodsub but I'm upgarding it to use gossip sub like the current version of the chat example.
See the github project for plans and futher information. 
https://github.com/esotericbyte/p2p-scope-rust



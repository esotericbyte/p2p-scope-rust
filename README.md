# p2p-scope-rust #

## Overview ##
This is a starter project for builiding an application using libp2p in Rust.
Currently this project extends the chat example from rust-libp2p with a terminal user interface. 
See https://github.com/libp2p/rust-libp2p/tree/master/examples/chat for further information about the chat example.

## Help rename project and/or exeutable ##
Something shorter would be better. 
* p2ps
* swarm_scope

## Goal ##
The goal of this project is to develop runtime tooling for libp2p oriented development, not a text chat though there will be some overlap of features.

## Current differences from the chat example ##
Instaed of mdns this version uses command line options to explicitly connect peers, it also uses floodsub in the current version intead of gossipsub. 

## Building and Using the executable ##
This section covers the basics. See the code for a complete list of options and thier current status.
Use cargo to build the executable file.
```cargo build``` 
```<path-to-exe> --help ```
Should be useful! 

To start the swarm you can run the executable and note it's listening address Multiaddr. Lanuch a second terminal or on another pc and use the --dial option with the listening multiaddr from the first or subsequent peer.

### Solving Known Issues ###
If you don't see information you are looking for, like listening addresses, resize the terminal window.

Data in the terminal interface is not curently selectalbe.

If using cywin or git-bash on windows prefix the multiaddr with aa backslash to prevent file name expansion.

## Runtime Executable Architechture ##

Libp2p applicationts in rust have a primary asyncronous event loop. 
This app runs a terminal interface with a syncronous event loop in a separate thread and a Tokio asyncronous event loop for networking. 

The asyncronous loop should be the primary loop for further development, or add adtional threads.
There is a generic intermediate API for the UI  so that the TUI can be easily subsituted or used with other options like Tauri. 

## Road Map ##

See the github project for plans and futher information. 
https://github.com/esotericbyte/p2p-scope-rust


#### Smaller Planned Changes ####

Add runtime and settings files for the application.

Add copy and paste.

The current version uses floodsub but I'm upgrading it to use gossipsub by default like the current version of the chat example.

Add a KAD-DHT.

Add hole punching.

Add a parser for scope commmands during runtime. These commands should introspect into a running node, and dataflows between nodes, and properties from a systems perspective. This feature is intended to be diagonostic. Make runtime command system cloneable for release application buids. 

Add ipfs pinning, and exchange of hashes between nodes to demonstrate how to use lib2p2 with ipfs. Maybe not so small but I think so. 

####  Swarm Config ####
Think about options and commands to configure and test libp2p swarms. By default the idea is to just change the code for the swarm or swarms the application uses.
Scope aims to co-exist with applicaiton UI and architecture.

#### Scope build target ####
Add a Scope build target that includes the scope UI, scope sub comand with options and rundtime commands. 
Libp2p networking configuration, event loop, an other parts of an application can be built separately in a release build target without the scope elements.

#### Tauri Interface ####
Develop a Tauri interface as an option and integrate it. This should be useable as a replacement or in paralel to the terminal interface. 

#### Lanuch Larger Derived project. ####
Fork the project with Tauri or similar rich interface and swarm configuation. Planing and community development needs to be done before this phase. 
Develop a modular framework for building services for local first and distributed resources.  
Disbribute html5 based documents and reference data.
Dataflows into the system and delevery from the system to cloud based applications using a Kafka style data model.
Contribute services such as identity, ui context, consensus for shared data. 
Model to Integrate other languages. 

## Primer Ideas for further development and forks. ##
**More ideas beyond current plans.**

Pick another UI lib or frameworka and build out an interface using it. 

### Chat App ###
Develop a richer libp2p chat app with a terminal interface such as markdown message support, account registartion or federation.  

### Networking and Consensus Models ###
Create a model of a libp2p swarm. Some nodes might take a "swarm_observer" role and listen on the network can maintain for informational messages collected from all nodes to update the model and enable key states, data flows, and networking operations of test or demonstrations to be collected and published as a dynamic document.


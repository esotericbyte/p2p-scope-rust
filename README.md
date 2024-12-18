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

To start the swarm run the executable and note it's listening address Multiaddr. To add a node to the swarm lanuch a second terminal and use the --dial option with the listening multiaddr from the first or subsequent peer.  For now it should work on the same lan and open internet addresses. 


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
The current version uses floodsub but I'm upgrading it to use gossipsub by default like the current version of the chat example.

Add a parser for scope commmands during runtime. These commands should introspect into a running node, and dataflows between nodes, and properties from a systems perspective. This feature is intended to be diagonostic. Make runtime command system cloneable for release application buids. 

Runtime and settings files for the application.

Implimentation of mdns as an option that advertizes listening ports, and allows them to be collected and dialed. 
I'm not convinced that mdns should be a swarm beahaivor. 

Copy and paste from the terminal interface in general or for specific data like listening addresses.

Develop the swarm behavivors: KAD-DHT, hole punching.

IPFS pinning, and exchange of hashes between nodes to demonstrate how to use lib2p2 with ipfs. 

####  Swarm Config ####
Think about options and commands to configure and test libp2p swarms. The tooling needs to match the network configuration in the swarm. 
How can the primary swarm and a scope be developed together and kept in sync? 
Are there projects that use multiple swarms?
Modularity could be served to use multiple processes and bridge them together through networking, message channels. 

#### Scope build target ####
Build targets with and without scope tooling. 
"Scope" that includes the scope UI, scope sub comand with options and runtime commands. 
"Release" build without the scope elements.

#### Tauri Interface ####
Develop a Tauri interface and integrate it.  
Having two interfaces active will require further development of the facade layer of fuctions so both working together might be another project.

## Primer Ideas for further development and forks. ##
**More ideas beyond current plans.**

Pick another UI lib or framework and build out an interface using it. 
Add p2p-scope-rust to an existing project to gain some insight into internals. 

### Chat App ###
Develop a richer libp2p chat app with a terminal interface such as markdown message support,images, emoji, account registartion, federation, web of trust.
Creating a similar project using the libp2p daemon go project together with python or Go for example could be interesting. 


### Networking and Consensus Models ###
Create a model of a libp2p swarm. Some nodes might take a "swarm_observer" role and listen on the network can maintain for informational messages collected from all nodes to update the model and enable key states, data flows, and networking operations of test or demonstrations to be collected and published as a dynamic document.



# rstream

rstream is a prototype implementation of a simple and secure web interface for monitoring pet/security camera video streams.

The general concept being explored in this prototype is the use of [WebRTC](https://webrtc.org) to stream individual camera feeds directly to a user's browser.

This project provides two components:

- A WebRTC signalling server, written in rust using the [Tokio](https://tokio.rs) framework, with websockets serving as the message transport.
- A front-end web app implemented in rust using the [Yew](https://yew.rs) framework and compiling to [WebAssembly](https://webassembly.org).

The third and currently unimplemented component would be the camera-side client component which negotiates connections with users via the signalling server and then converts camera output to WebRTC (this last part can be handled with gstreamer or similar projects). 

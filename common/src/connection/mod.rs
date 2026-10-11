//! T846: portable connection contracts. Native handles, authentication and
//! media ownership belong to adapters; parsing a frame grants no authority.
pub mod flow;
pub mod hello;
pub mod mux;
pub mod pairing;
pub mod queue;
pub mod ready;
pub mod wire;

pub mod management;
pub mod resources;
pub mod routes;

pub mod trust;

pub mod control;
pub mod control_client;
pub mod control_view;
pub mod endpoint;
pub mod registration;
pub mod window;

pub mod registration_client;

pub mod media;

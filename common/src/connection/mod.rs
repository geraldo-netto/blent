//! T846: portable connection contracts. Native handles, authentication and
//! media ownership belong to adapters; parsing a frame grants no authority.
pub mod flow;
pub mod mux;
pub mod queue;
pub mod wire;
pub mod pairing;
pub mod hello;
pub mod ready;

pub mod management;
pub mod resources;
pub mod routes;

pub mod trust;

pub mod registration;

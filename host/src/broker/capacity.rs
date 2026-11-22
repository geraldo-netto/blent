//! Portable broker admission defaults and validated upper bounds.
//! Registrations each consume one client; default spare clients fund controls.
pub const DEFAULT_CLIENTS: usize = 128;
pub const MAX_CLIENTS: usize = 1024;
pub const DEFAULT_LINKS: usize = 4;
pub const MAX_LINKS: usize = 64;
pub const DEFAULT_REGISTRATIONS: usize = 64;
pub const MAX_REGISTRATIONS: usize = 1024;

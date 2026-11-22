//! Immutable desktop appearance values, independent of GUI/OS implementations.
//! The native adapter validates its bounded input; UI consumes one owned snapshot.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Appearance {
    pub source: String,
    pub background: [u8; 3],
    pub surface: [u8; 3],
    pub foreground: [u8; 3],
    pub accent: [u8; 3],
    pub selected_text: [u8; 3],
    pub font_family: String,
    pub font_points: f32,
    pub text_scale: f32,
    pub animations: bool,
}

//! Owned, bounded scalar-property request; independent of D-Bus storage types.
use std::io;

pub(crate) enum Scalar {
    Text(String),
    Integer(i32),
}
pub(crate) struct Property {
    pub path: String,
    pub interface: String,
    pub name: String,
    pub value: Scalar,
}
impl Property {
    pub(crate) fn new(
        path: &str,
        interface: &str,
        name: &str,
        signature: &str,
        value: &str,
    ) -> io::Result<Self> {
        if [path, interface, name, value]
            .iter()
            .any(|field| field.len() > 4096)
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let value = match signature {
            "s" => Scalar::Text(value.to_owned()),
            "i" => Scalar::Integer(value.parse().map_err(|_| io::ErrorKind::InvalidInput)?),
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        };
        Ok(Self {
            path: path.to_owned(),
            interface: interface.to_owned(),
            name: name.to_owned(),
            value,
        })
    }
}

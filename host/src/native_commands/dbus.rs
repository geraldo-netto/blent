//! T1640: settle an original KWin reply, never infer acknowledgement from CLI exit.
use super::property::{Property, Scalar};
use std::io;
use zbus::{Connection, Message};

pub(super) struct Report {
    pub settled: bool,
    pub result: io::Result<()>,
}
impl Report {
    fn rejected() -> Self {
        Self {
            settled: true,
            result: Err(io::Error::other(
                "KWin property request rejected before dispatch",
            )),
        }
    }
}

pub(super) async fn execute(request: Property) -> Report {
    let connection = match connect().await {
        Ok(connection) => connection,
        Err(_) => return Report::rejected(),
    };
    let report = on_connection(&connection, request).await;
    // The client has no object-server callbacks or borrowed device storage.
    // Its original worker owns the connection through reply and local close.
    let _ = connection.close().await;
    report
}

async fn connect() -> zbus::Result<Connection> {
    zbus::connection::Builder::session()?
        .max_queued(1)
        .build()
        .await
}

fn valid(request: &Property) -> bool {
    zbus::zvariant::ObjectPath::try_from(request.path.as_str()).is_ok()
        && zbus::names::InterfaceName::try_from(request.interface.as_str()).is_ok()
        && zbus::names::MemberName::try_from(request.name.as_str()).is_ok()
}

async fn owner(connection: &Connection) -> zbus::Result<String> {
    let reply = connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &("org.kde.KWin",),
        )
        .await?;
    let name = reply.body().deserialize::<String>()?;
    zbus::names::UniqueName::try_from(name.as_str())?;
    Ok(name)
}

async fn on_connection(connection: &Connection, request: Property) -> Report {
    if !valid(&request) {
        return Report::rejected();
    }
    let original = match owner(connection).await {
        Ok(owner) => owner,
        Err(_) => return Report::rejected(),
    };
    let value = match &request.value {
        Scalar::Text(value) => zbus::zvariant::Value::from(value.as_str()),
        Scalar::Integer(value) => zbus::zvariant::Value::from(*value),
    };
    let result = connection
        .call_method(
            Some(original.as_str()),
            request.path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "Set",
            &(request.interface.as_str(), request.name.as_str(), value),
        )
        .await;
    classify(result, &original)
}

fn sender(reply: &Message, expected: &str) -> bool {
    reply
        .header()
        .sender()
        .is_some_and(|sender| sender.as_str() == expected)
}

fn undispatched(name: &str, reply: &Message) -> bool {
    sender(reply, "org.freedesktop.DBus")
        && matches!(
            name,
            "org.freedesktop.DBus.Error.ServiceUnknown"
                | "org.freedesktop.DBus.Error.NameHasNoOwner"
                | "org.freedesktop.DBus.Error.AccessDenied"
        )
}

fn classify(result: zbus::Result<Message>, original: &str) -> Report {
    let settled = match &result {
        Ok(reply) => sender(reply, original),
        Err(zbus::Error::MethodError(name, _, reply)) => {
            sender(reply, original) || undispatched(name.as_str(), reply)
        }
        Err(_) => false,
    };
    let result = if settled && result.is_ok() {
        Ok(())
    } else {
        Err(io::Error::other(
            "KWin property set failed or its original reply was lost",
        ))
    };
    Report { settled, result }
}

#[cfg(test)]
mod tests;

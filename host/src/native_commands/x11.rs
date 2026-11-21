//! T1641: a single original X11 connection owns the mapping request and its fence.
use super::mutation::Report;
use randr::ConnectionExt as _;
use std::io;
use x11rb::protocol::{randr, xinput, xproto};
use x11rb::{connection::Connection, errors::ReplyError, rust_connection::RustConnection};
use xinput::ConnectionExt as _;
use xproto::ConnectionExt as _;
mod transform;
use transform::{Geometry, Orientation, Turn};

use super::mapping::Mapping;
struct Prepared {
    device: u16,
    property: u32,
    float: u32,
    matrix: [f32; 9],
}
pub(super) async fn execute(request: Mapping) -> Report {
    joined(move || run(request)).await
}
async fn joined(work: impl FnOnce() -> Report + Send + 'static) -> Report {
    match tokio::task::spawn_blocking(work).await {
        Ok(report) => report,
        Err(_) => Report {
            settled: false,
            result: Err(io::Error::other("original X11 worker lost")),
        },
    }
}
fn run(request: Mapping) -> Report {
    let Ok((connection, screen)) = RustConnection::connect(None) else {
        return Report::rejected();
    };
    match prepare(&connection, screen, request) {
        Ok(prepared) => apply(&connection, prepared),
        Err(_) => Report::rejected(),
    }
}
fn prepare(
    connection: &RustConnection,
    screen: usize,
    request: Mapping,
) -> anyhow::Result<Prepared> {
    connection.xinput_xi_query_version(2, 0)?.reply()?;
    let root = connection.setup().roots[screen].root;
    let matrix = output_matrix(connection, root, &request.output)?;
    let device = device(connection, &request.device)?;
    let float = atom(connection, b"FLOAT")?;
    let property = atom(connection, b"Coordinate Transformation Matrix")?;
    let current = connection
        .xinput_xi_get_property(device, false, property, float, 0, 9)?
        .reply()?;
    if !valid_property(&current, float) {
        anyhow::bail!("X11 matrix property is unavailable");
    }
    Ok(Prepared {
        device,
        property,
        float,
        matrix,
    })
}
fn atom(connection: &RustConnection, name: &[u8]) -> anyhow::Result<u32> {
    let atom = connection.intern_atom(true, name)?.reply()?.atom;
    if atom == 0 {
        anyhow::bail!("required X11 atom unavailable");
    }
    Ok(atom)
}
fn device(connection: &RustConnection, name: &str) -> anyhow::Result<u16> {
    let reply = connection
        .xinput_xi_query_device(xinput::Device::ALL)?
        .reply()?;
    let mut matches = reply
        .infos
        .iter()
        .filter(|device| device.name == name.as_bytes());
    let matched = matches
        .next()
        .ok_or_else(|| anyhow::anyhow!("X11 device absent"))?;
    if matches.next().is_some() {
        anyhow::bail!("X11 device name is ambiguous");
    }
    Ok(matched.deviceid)
}
fn valid_property(reply: &xinput::XIGetPropertyReply, float: u32) -> bool {
    reply.type_ == float
        && reply.bytes_after == 0
        && reply.num_items == 9
        && matches!(&reply.items, xinput::XIGetPropertyItems::Data32(values) if values.len() == 9)
}
fn orientation(rotation: randr::Rotation) -> anyhow::Result<Orientation> {
    let bits = u16::from(rotation);
    if bits & !63 != 0 {
        anyhow::bail!("unsupported RandR orientation flags");
    }
    let turn = match bits & 15 {
        1 => Turn::Zero,
        2 => Turn::Left,
        4 => Turn::Half,
        8 => Turn::Right,
        _ => anyhow::bail!("invalid RandR rotation"),
    };
    Ok(Orientation {
        turn,
        reflect_x: bits & 16 != 0,
        reflect_y: bits & 32 != 0,
    })
}
fn output_crtc(
    connection: &RustConnection,
    root: u32,
    name: &str,
) -> anyhow::Result<randr::GetCrtcInfoReply> {
    let resources = connection
        .randr_get_screen_resources_current(root)?
        .reply()?;
    for output in resources.outputs {
        let info = connection
            .randr_get_output_info(output, resources.config_timestamp)?
            .reply()?;
        if info.name == name.as_bytes() {
            if !(info.status == randr::SetConfig::SUCCESS
                && info.connection == randr::Connection::CONNECTED
                && info.crtc != 0)
            {
                anyhow::bail!("X11 output is not active");
            }
            let crtc = connection
                .randr_get_crtc_info(info.crtc, resources.config_timestamp)?
                .reply()?;
            if crtc.status != randr::SetConfig::SUCCESS {
                anyhow::bail!("X11 output configuration changed");
            }
            return Ok(crtc);
        }
    }
    anyhow::bail!("X11 output is absent")
}
fn output_matrix(connection: &RustConnection, root: u32, name: &str) -> anyhow::Result<[f32; 9]> {
    let crtc = output_crtc(connection, root, name)?;
    let canvas = connection.get_geometry(root)?.reply()?;
    transform::matrix(Geometry {
        canvas: (canvas.width, canvas.height),
        rectangle: (crtc.x, crtc.y, crtc.width, crtc.height),
        orientation: orientation(crtc.rotation)?,
    })
    .ok_or_else(|| anyhow::anyhow!("X11 output has empty geometry"))
}
fn apply(connection: &RustConnection, prepared: Prepared) -> Report {
    let values = xinput::XIChangePropertyAux::Data32(prepared.matrix.map(f32::to_bits).to_vec());
    let result = connection
        .xinput_xi_change_property(
            prepared.device,
            xproto::PropMode::REPLACE,
            prepared.property,
            prepared.float,
            9,
            &values,
        )
        .map_err(ReplyError::from)
        .and_then(|request| request.check());
    classify(result)
}
fn classify(result: Result<(), ReplyError>) -> Report {
    let settled = matches!(&result, Ok(()) | Err(ReplyError::X11Error(_)));
    Report {
        settled,
        result: result.map_err(|_| {
            io::Error::other("X11 mapping failed or its original acknowledgement was lost")
        }),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod transform_tests;

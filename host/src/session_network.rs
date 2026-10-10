//! Native TCP composition adapter. These capabilities are deliberately separate
//! from the portable capture, input, attachment and session lifecycle contracts.
use anyhow::Result;
use std::net::TcpListener;

/// Owned listener duplicates pin the actual bound ports, including ephemeral
/// ports. A media adapter retains them through its own registration retirement;
/// session-task retirement alone does not revoke duplicated native ownership.
pub struct RetainedListeners {
    video: TcpListener,
    control: TcpListener,
    ports: (u16, u16),
}
impl RetainedListeners {
    pub(crate) fn retain(
        video: tokio::net::TcpListener,
        control: tokio::net::TcpListener,
    ) -> Result<(Self, tokio::net::TcpListener, tokio::net::TcpListener)> {
        let ports = (video.local_addr()?.port(), control.local_addr()?.port());
        let retained = Self {
            video: video.into_std()?,
            control: control.into_std()?,
            ports,
        };
        let video = tokio::net::TcpListener::from_std(retained.video.try_clone()?)?;
        let control = tokio::net::TcpListener::from_std(retained.control.try_clone()?)?;
        Ok((retained, video, control))
    }
    pub fn ports(&self) -> (u16, u16) {
        self.ports
    }
    /// Video then control, matching the independent display-registration wire
    /// contract. Never accept on these duplicates; the session owns accept loops.
    pub fn try_clone(&self) -> Result<Vec<TcpListener>> {
        Ok(vec![self.video.try_clone()?, self.control.try_clone()?])
    }
}

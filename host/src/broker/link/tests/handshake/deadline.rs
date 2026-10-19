//! T1105: real broker READY/TLS propagation with independent stage delays.
use super::*;

async fn peer_after_ready(
    mut remote: tokio::io::DuplexStream,
    tablet: Identity,
    host: Arc<Identity>,
    ready_delay: Duration,
    tls_delay: Duration,
) -> Result<()> {
    let mut marker = [0; ready::BYTES];
    remote.read_exact(&mut marker).await?;
    ready::decode(&marker, true)?;
    tokio::time::sleep(ready_delay).await;
    remote.write_all(&ready::encode(false)).await?;
    remote.flush().await?;
    tokio::time::sleep(tls_delay).await;
    let approved = PeerCertificate::parse(host.certificate().to_vec())?;
    let mut tls = secure_connection::connect(
        remote,
        &tablet,
        &approved,
        blent_config::connection::handshake::Timeout::default(),
    )
    .await?;
    let mut window = [0; protocol_window::BYTES];
    tls.read_exact(&mut window).await?;
    assert_eq!(protocol_window::decode(&window, true)?, 4096);
    tls.write_all(&protocol_window::encode(false, 4096)?)
        .await?;
    tls.flush().await?;
    Ok(())
}
async fn selected_budget(ready_delay: Duration, tls_delay: Duration, expires: bool) {
    let (resource, mut remote) = resource();
    let mut options = settings();
    options.limits.channel_bytes = 4096;
    options.tls_timeout =
        blent_config::connection::handshake::Timeout::new(Duration::from_millis(37)).unwrap();
    let host = options.identity.clone();
    let tablet = Identity::load(IdentityRecord::generate().unwrap()).unwrap();
    let mut handle = Handle::start(resource, options).unwrap();
    let native = handle.native_completion();
    let task = handle.task_completion();
    remote_hello(&mut remote, tablet.certificate()).await;
    let Event::Identified(peer) = notice(&mut handle).await else {
        panic!("T1105 missing original public identity");
    };
    assert!(handle
        .submit(Command::Approve(Ticket { epoch: 1, peer }))
        .is_ok());
    let (outcome, peer) = tokio::join!(
        handle.next(),
        peer_after_ready(remote, tablet, host, ready_delay, tls_delay)
    );
    let expired = outcome
        .as_ref()
        .err()
        .is_some_and(|error| error.is::<tokio::time::error::Elapsed>());
    drop(outcome); // Includes an unexpected Verified/native owner on faulty paths.
    handle.stop();
    drop(handle);
    // Original retirement reaper is a real thread, independent of virtual time.
    tokio::time::resume();
    completed(&native).await;
    completed(&task).await;
    assert_eq!(
        expired, expires,
        "T1105 original broker TLS budget ignored or applied to approval"
    );
    assert_eq!(peer.is_err(), expires);
}
#[tokio::test(start_paused = true)]
async fn t1105_original_link_uses_selected_tls_budget_after_ready() {
    selected_budget(Duration::ZERO, Duration::from_millis(100), true).await;
}
#[tokio::test(start_paused = true)]
async fn t1105_ready_approval_wait_does_not_consume_new_tls_budget() {
    selected_budget(Duration::from_millis(100), Duration::ZERO, false).await;
}

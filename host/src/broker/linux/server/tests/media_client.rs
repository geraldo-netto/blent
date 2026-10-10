use super::*;
use super::{
    registration::{exchange, listeners},
    tls::authenticated,
};
use crate::broker::linux::{client::Launch, registration_client};
use blent_config::connection::{
    management::{Credential, Profile},
    registration_client::{Client, Phase, Request},
};

#[tokio::test]
async fn t911_registration_client_uses_real_broker_tls_and_retained_descriptors() {
    registration_fixture(false).await;
}

#[tokio::test]
async fn t925_bound_registration_uses_real_broker_tls_and_retained_ports() {
    registration_fixture(true).await;
}

async fn registration_fixture(bound: bool) {
    let auth = authenticated().await;
    let settings = registration_client::Settings::new(Launch {
        executable: "/bin/false".into(),
        runtime: auth.fixture.server.settings.runtime.clone(),
        state: auth.fixture.server.settings.state.clone(),
        helper: auth.fixture.server.settings.helper.clone(),
    });
    let factory = std::sync::Arc::new(registration_client::Factory::new(settings).unwrap());
    let (stop, done) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let mut fixture = auth.fixture;
        tokio::select! { result=fixture.server.run()=>panic!("server failed: {result:?}"), _=done=>() }
        fixture
    });
    let (video, control) = listeners();
    let request = Request {
        peer: auth.pin,
        resource: Resource::DisplaySession,
        credential: Credential::new([9; 32]),
        profile: Profile::Display,
    };
    let client: Box<dyn Client> = if bound {
        use crate::registered_display::Registration;
        let (retained, _video, _control) = crate::session_network::RetainedListeners::retain(
            tokio::net::TcpListener::from_std(video.try_clone().unwrap()).unwrap(),
            tokio::net::TcpListener::from_std(control.try_clone().unwrap()).unwrap(),
        )
        .unwrap();
        let mut bound = crate::broker::linux::display_registration::Bound::new(
            factory,
            std::sync::Arc::new(retained),
        );
        bound.start(request).unwrap()
    } else {
        Box::new(
            factory
                .start(
                    request,
                    vec![video.try_clone().unwrap(), control.try_clone().unwrap()],
                )
                .unwrap(),
        )
    };
    let tablet = tokio::spawn(crate::broker::session::tests::tls::tablet(auth.tablet));
    let (_sockets, tablet) =
        bounded(async { tokio::join!(exchange(video, control), tablet) }).await;
    let tablet = tablet.unwrap();
    bounded(async {
        while client.status().phase != Phase::Active {
            tokio::task::yield_now().await;
        }
    })
    .await;
    client.close();
    assert_ne!(client.status().phase, Phase::Active);
    assert!(!client.retired());
    // Disconnected native TLS peer must retire broker bridges before proof.
    drop(tablet);
    bounded(async {
        while !client.retired() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await;
    assert!(client.status().remote_retired);
    stop.send(()).unwrap();
    let mut fixture = bounded(running).await.unwrap();
    assert!(fixture.server.routes.is_empty());
    fixture
        .server
        .shutdown(Duration::from_secs(2))
        .await
        .unwrap();
    fixture.finish().await;
}

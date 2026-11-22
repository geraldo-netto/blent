//! Fixed scheduling hints, not resource authority. Monotonic IDs let a bounded
//! round finish even if newer clients arrive continuously. No owner is cloned.
use super::{Client, Input, Link};
use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Included},
    task::{Context, Poll},
};

#[derive(Default)]
struct Cursor {
    after: u64,
    ceiling: u64,
}
impl Cursor {
    fn poll<T, R>(
        &mut self,
        records: &mut BTreeMap<u64, T>,
        poll: fn(u64, &mut T, &mut Context<'_>) -> Poll<R>,
        cx: &mut Context<'_>,
    ) -> Poll<R> {
        if let Poll::Ready((id, value)) = entries(
            records.range_mut((Excluded(self.after), Included(self.ceiling))),
            poll,
            cx,
        ) {
            self.after = id;
            return Poll::Ready(value);
        }
        let previous = (self.after, self.ceiling);
        self.ceiling = records.last_key_value().map_or(0, |(&id, _)| id);
        self.after = 0;
        // Earlier IDs and new arrivals were not polled above. Poll each owner
        // at most once this call; new arrivals join this newly bounded round.
        entries(
            records
                .iter_mut()
                .filter(|(id, _)| **id <= previous.0 || **id > previous.1),
            poll,
            cx,
        )
        .map(|(id, value)| {
            self.after = id;
            value
        })
    }
}
fn entries<'a, T: 'a, R>(
    records: impl Iterator<Item = (&'a u64, &'a mut T)>,
    poll: fn(u64, &mut T, &mut Context<'_>) -> Poll<R>,
    cx: &mut Context<'_>,
) -> Poll<(u64, R)> {
    for (&id, record) in records {
        if let Poll::Ready(value) = poll(id, record, cx) {
            return Poll::Ready((id, value));
        }
    }
    Poll::Pending
}

#[derive(Default)]
pub(super) struct Readiness {
    clients: Cursor,
    links: Cursor,
    links_first: bool,
}
impl Readiness {
    pub fn poll(
        &mut self,
        clients: &mut BTreeMap<u64, Client>,
        links: &mut BTreeMap<u64, Link>,
        cx: &mut Context<'_>,
    ) -> Poll<Input> {
        let selected = self.ordered(clients, links, cx);
        if selected.is_ready() {
            self.links_first = !self.links_first;
        }
        selected
    }
    fn ordered(
        &mut self,
        clients: &mut BTreeMap<u64, Client>,
        links: &mut BTreeMap<u64, Link>,
        cx: &mut Context<'_>,
    ) -> Poll<Input> {
        if self.links_first {
            if let Poll::Ready(input) = self.links.poll(links, link, cx) {
                return Poll::Ready(input);
            }
            return self.clients.poll(clients, client, cx);
        }
        if let Poll::Ready(input) = self.clients.poll(clients, client, cx) {
            return Poll::Ready(input);
        }
        self.links.poll(links, link, cx)
    }
}
fn client(id: u64, record: &mut Client, cx: &mut Context<'_>) -> Poll<Input> {
    match &mut record.peer {
        Some(peer) => peer.poll_next(cx).map(|result| Input::Client(id, result)),
        None => Poll::Pending,
    }
}
fn link(id: u64, record: &mut Link, cx: &mut Context<'_>) -> Poll<Input> {
    match &mut record.handle {
        Some(handle) => handle.poll_next(cx).map(|result| Input::Link(id, result)),
        None => Poll::Pending,
    }
}

#[cfg(test)]
mod tests;

use super::*;
use crate::{document::Document, world::WorldBounds};
use std::{net::TcpListener, rc::Rc, time::Duration};
use geometry_math::vec::Vec3d;

fn pair() -> (Session, Session) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let guest = Session::join(&addr.to_string()).unwrap();
    let (stream, _) = listener.accept().unwrap();
    let host = Session::new(
        Link::from_accepted(stream).unwrap(),
        Origin::TcpHost(addr.port()),
    );
    (host, guest)
}

fn wired(session: Rc<Session>, world: &World) -> Document {
    let mut doc = Document::new(WorldBounds::default());
    doc.adopt_world(world.clone());
    doc.set_on_edit(Rc::new(move |edit| session.send_edit(edit)));
    doc
}

fn add(doc: &mut Document) {
    let brush =
        map_model::shapes::create_cuboid(&doc.world_bounds(), &Bbox3d::from_scalars(-8., 8.), "stone")
            .unwrap();
    doc.add_brushes(vec![brush]);
}

fn pump(peers: &mut [(&Session, &mut Document)]) {
    for _ in 0..80 {
        for (session, doc) in peers.iter_mut() {
            for event in session.poll(&doc.world_bounds()) {
                match event {
                    SessionEvent::ApplyEdit(edit) => doc.apply_remote(&edit),
                    SessionEvent::AdoptWorld(world) => doc.adopt_world(world),
                    SessionEvent::PeerCamera { peer, position, direction } => doc.set_peer_camera(peer, Some((position, direction))),
                    SessionEvent::PeerName { peer, name } => doc.set_peer_name(peer, name),
                    SessionEvent::PeerLeft { peer, .. } => doc.remove_peer(peer),
                    SessionEvent::ConnectionLost => doc.clear_peers(),
                    _ => {}
                }
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn concurrent_edits_of_one_brush_converge_in_host_order() {
    let (host, first) = pair();
    let (joined, second) = pair();
    host.add_peer(joined);
    let host = Rc::new(host);
    let first = Rc::new(first);
    let second = Rc::new(second);
    let mut seed = Document::new(WorldBounds::default());
    add(&mut seed);
    let mut h = wired(host.clone(), seed.tree_world());
    let mut a = wired(first.clone(), seed.tree_world());
    let mut b = wired(second.clone(), seed.tree_world());
    h.select_all();
    a.select_all();
    b.select_all();
    h.translate_selection(Vec3d::new(4., 0., 0.));
    a.translate_selection(Vec3d::new(16., 0., 0.));
    b.translate_selection(Vec3d::new(32., 0., 0.));
    pump(&mut [(&host, &mut h), (&first, &mut a), (&second, &mut b)]);
    assert_eq!(h.to_map_text(), a.to_map_text());
    assert_eq!(h.to_map_text(), b.to_map_text());
}

#[test]
fn undo_and_redo_are_shared_and_add_echo_does_not_duplicate_nodes() {
    let (host, guest) = pair();
    let host = Rc::new(host);
    let guest = Rc::new(guest);
    let seed = Document::new(WorldBounds::default());
    let mut h = wired(host.clone(), seed.tree_world());
    let mut g = wired(guest.clone(), seed.tree_world());
    add(&mut g);
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(map_model::node::brushes_with_ids(g.tree_world()).len(), 1);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.undo().unwrap();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.redo().unwrap();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    assert_eq!(map_model::node::brushes_with_ids(g.tree_world()).len(), 1);
}

#[test]
fn a_guest_cannot_replace_the_host_world() {
    let (host, guest) = pair();
    let seed = Document::new(WorldBounds::default());
    guest.send_world(seed.tree_world());
    let mut received = false;
    for _ in 0..80 {
        let events = host.poll(&seed.world_bounds());
        assert!(!events
            .iter()
            .any(|e| matches!(e, SessionEvent::AdoptWorld(_))));
        received |= events
            .iter()
            .any(|e| matches!(e, SessionEvent::Undecodable(_)));
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(received);
}

#[test]
fn stopping_a_host_releases_its_port_without_another_connection() {
    let (address, inbox) = Session::host_on_this_network(0).unwrap();
    let port = address.rsplit(':').next().unwrap().parse::<u16>().unwrap();
    assert_ne!(port, 0);
    drop(inbox);
    for _ in 0..100 {
        if TcpListener::bind(("0.0.0.0", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("host port remained bound after leaving the session");
}

#[test]
fn edits_without_acknowledgement_survive_disconnect_and_replay_once() {
    let (host, guest) = pair();
    let host = Rc::new(host);
    let guest = Rc::new(guest);
    let seed = Document::new(WorldBounds::default());
    let mut h = wired(host.clone(), seed.tree_world());
    let mut g = wired(guest.clone(), seed.tree_world());
    // The host has not polled or acknowledged this edit when the link goes away.
    add(&mut g);
    let pending = guest.take_unacknowledged();
    assert_eq!(pending.len(), 1);
    drop(host);
    let (new_host, new_guest) = pair();
    let new_host = Rc::new(new_host);
    let new_guest = Rc::new(new_guest);
    h.clear_on_edit();
    h.set_on_edit(Rc::new({
        let s = new_host.clone();
        move |e| s.send_edit(e)
    }));
    g.adopt_world(h.tree_world().clone());
    g.set_on_edit(Rc::new({
        let s = new_guest.clone();
        move |e| s.send_edit(e)
    }));
    for edit in &pending {
        g.apply_remote(edit);
        new_guest.send_edit(edit);
    }
    pump(&mut [(&new_host, &mut h), (&new_guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    assert_eq!(map_model::node::brushes_with_ids(g.tree_world()).len(), 1);
    assert!(new_guest.take_unacknowledged().is_empty());
    // Replay is safe even if the host got the add but its acknowledgement was lost.
    for edit in &pending {
        g.apply_remote(edit);
        new_guest.send_edit(edit);
    }
    pump(&mut [(&new_host, &mut h), (&new_guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    assert_eq!(map_model::node::brushes_with_ids(h.tree_world()).len(), 1);
}


#[test]
fn stationary_cameras_reach_late_joiners_and_disappear_on_departure() {
    let (host, first) = pair();
    let mut h = Document::new(WorldBounds::default());
    let mut a = Document::new(WorldBounds::default());
    let original = h.to_map_text();
    let hp = Vec3d::new(100., 200., 64.);
    let ap = Vec3d::new(-80., 32., 128.);
    let direction = Vec3d::new(0., 0., 1.);
    host.send_camera(hp, direction);
    first.send_camera(ap, direction);
    first.send_name("Alice");
    pump(&mut [(&host, &mut h), (&first, &mut a)]);
    assert_eq!(h.peer_cameras().get(&1), Some(&(ap, direction)));
    assert_eq!(a.peer_cameras().get(&0), Some(&(hp, direction)));
    assert!(a.peer_cameras().get(&1).is_none(), "own camera is not echoed");
    let (joined, second) = pair();
    let second_id = host.add_peer(joined);
    let mut b = Document::new(WorldBounds::default());
    host.send_world_to(second_id, h.tree_world());
    pump(&mut [(&host, &mut h), (&first, &mut a), (&second, &mut b)]);
    assert_eq!(b.peer_cameras().len(), 2);
    assert_eq!(b.peer_cameras().get(&0), Some(&(hp, direction)));
    assert_eq!(b.peer_cameras().get(&1), Some(&(ap, direction)));
    assert_eq!(b.peer_name(1), "Alice");
    assert_eq!(h.to_map_text(), original, "presence is not map content");
    first.send_camera(ap + direction * 32., Vec3d::new(1., 0., 0.));
    pump(&mut [(&host, &mut h), (&first, &mut a), (&second, &mut b)]);
    assert_eq!(b.peer_cameras().get(&1), Some(&(ap + direction * 32., Vec3d::new(1., 0., 0.))));
    drop(first);
    pump(&mut [(&host, &mut h), (&second, &mut b)]);
    assert!(!b.peer_cameras().contains_key(&1));
    assert!(!host.cameras.borrow().contains_key(&1));
    assert_eq!(b.peer_name(1), "Peer 1");
}

use super::*;
use crate::{document::Document, world::WorldBounds};
use geometry_math::vec::Vec3d;
use std::{net::TcpListener, rc::Rc, time::Duration};

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
    doc.set_on_edit(Rc::new(move |edit, layers| {
        session.send_edit_in(edit, layers)
    }));
    doc
}

fn add(doc: &mut Document) {
    let brush = map_model::shapes::create_cuboid(
        &doc.world_bounds(),
        &Bbox3d::from_scalars(-8., 8.),
        "stone",
    )
    .unwrap();
    doc.add_brushes(vec![brush]);
}

fn pump(peers: &mut [(&Session, &mut Document)]) {
    for _ in 0..80 {
        for (session, doc) in peers.iter_mut() {
            for event in session.poll(&doc.world_bounds()) {
                match event {
                    SessionEvent::ApplyEdit(edit, targets) => doc.apply_remote_to(&edit, &targets),
                    SessionEvent::AdoptWorld(world) => doc.adopt_world(world),
                    SessionEvent::PeerCamera {
                        peer,
                        position,
                        direction,
                    } => doc.set_peer_camera(peer, Some((position, direction))),
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
        move |e, layers| s.send_edit_in(e, layers)
    }));
    g.adopt_world(h.tree_world().clone());
    g.set_on_edit(Rc::new({
        let s = new_guest.clone();
        move |e, layers| s.send_edit_in(e, layers)
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
    assert!(
        a.peer_cameras().get(&1).is_none(),
        "own camera is not echoed"
    );
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
    assert_eq!(
        b.peer_cameras().get(&1),
        Some(&(ap + direction * 32., Vec3d::new(1., 0., 0.)))
    );
    drop(first);
    pump(&mut [(&host, &mut h), (&second, &mut b)]);
    assert!(!b.peer_cameras().contains_key(&1));
    assert!(!host.cameras.borrow().contains_key(&1));
    assert_eq!(b.peer_name(1), "Peer 1");
}

#[test]
fn guest_layer_add_remove_and_undo_echo_preserve_other_layer_geometry() {
    let (host, guest) = pair();
    let host = Rc::new(host);
    let guest = Rc::new(guest);
    let mut seed = Document::new(Default::default());
    seed.add_layer("A");
    add(&mut seed);
    seed.add_layer("B");
    add(&mut seed);
    let mut h = wired(host.clone(), seed.tree_world());
    let mut g = wired(guest.clone(), seed.tree_world());
    g.add_layer("C");
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.layers().len(), 4);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.undo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.layers().len(), 3);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.redo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.layers().len(), 4);
    assert_eq!(h.to_map_text(), g.to_map_text());
    let before = g.to_map_text();
    assert!(g.remove_layer(1));
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.layers().len(), 3);
    assert_eq!(g.brush_count(), 2);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.undo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.to_map_text(), before);
    assert_eq!(h.to_map_text(), g.to_map_text());
    g.redo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(g.brush_count(), 2);
    assert_eq!(h.to_map_text(), g.to_map_text());
}

#[test]
fn concurrent_layer_insertions_reconcile_order_and_keep_optimistic_contents() {
    let (host, first) = pair();
    let (joined, second) = pair();
    host.add_peer(joined);
    let host = Rc::new(host);
    let first = Rc::new(first);
    let second = Rc::new(second);
    let seed = Document::new(Default::default());
    let mut h = wired(host.clone(), seed.tree_world());
    let mut a = wired(first.clone(), seed.tree_world());
    let mut b = wired(second.clone(), seed.tree_world());
    a.add_layer("A");
    add(&mut a);
    b.add_layer("B");
    add(&mut b);
    pump(&mut [(&host, &mut h), (&first, &mut a), (&second, &mut b)]);
    assert_eq!(h.layers().len(), 3);
    assert_eq!(h.brush_count(), 2);
    assert_eq!(h.to_map_text(), a.to_map_text());
    assert_eq!(h.to_map_text(), b.to_map_text());
    a.rename_layer(1, "renamed");
    pump(&mut [(&host, &mut h), (&first, &mut a), (&second, &mut b)]);
    assert_eq!(h.to_map_text(), a.to_map_text());
    assert_eq!(h.to_map_text(), b.to_map_text());
}

#[test]
fn layer_export_omission_is_shared_with_existing_guests() {
    let (host, guest) = pair();
    let host = Rc::new(host);
    let guest = Rc::new(guest);
    let mut seed = Document::new(WorldBounds::default());
    seed.add_layer("Notes");
    let mut h = wired(host.clone(), seed.tree_world());
    let mut g = wired(guest.clone(), seed.tree_world());
    assert!(h.set_layer_omitted(1, true));
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert!(h.tree_world().layers[1].omit_from_export);
    assert!(g.tree_world().layers[1].omit_from_export);
    assert_eq!(h.to_map_text(), g.to_map_text());

    // It is one undo step, and the undo reaches the guest too.
    h.undo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert!(!h.tree_world().layers[1].omit_from_export);
    assert!(!g.tree_world().layers[1].omit_from_export);
}

#[test]
fn a_remote_add_replaces_a_nested_node_with_the_same_id() {
    let mut doc = Document::new(WorldBounds::default());
    add(&mut doc);
    let (brush_id, brush) = map_model::node::brushes_with_ids(doc.tree_world())
        .first()
        .map(|(id, brush)| (*id, (*brush).clone()))
        .unwrap();
    // Another editor grouped the brush: the group is new, the brush in it is not.
    let group =
        map_model::node::Node::new(map_model::node::NodeKind::Group(map_model::node::Group {
            id: "9".into(),
            name: "Box".into(),
            linked_group_id: None,
            transformation: None,
            children: vec![map_model::node::Node::with_id(
                brush_id,
                map_model::node::NodeKind::Brush(brush),
            )],
        }));
    doc.apply_remote(&crate::history::Edit::Added {
        layer: 0,
        nodes: vec![group],
    });
    assert_eq!(map_model::node::brushes_with_ids(doc.tree_world()).len(), 1);
}

/// A host and a guest on one world with two extra layers, `a` and `b`.
fn layered() -> (Rc<Session>, Rc<Session>, Document, Document) {
    let (host, guest) = pair();
    let (host, guest) = (Rc::new(host), Rc::new(guest));
    let mut seed = Document::new(WorldBounds::default());
    seed.add_layer("a");
    seed.add_layer("b");
    let h = wired(host.clone(), seed.tree_world());
    let g = wired(guest.clone(), seed.tree_world());
    (host, guest, h, g)
}

fn on_default_layer(doc: &Document) -> usize {
    map_model::node::brushes_by_layer(doc.tree_world())
        .iter()
        .filter(|(_, _, layer)| *layer == 0)
        .count()
}

#[test]
fn concurrent_layer_removal_keeps_a_new_brush_on_its_surviving_layer() {
    // The host removes layer a while the guest adds to layer b, which then moves up one.
    let (host, guest, mut h, mut g) = layered();
    assert!(h.remove_layer(1));
    g.set_current_layer(2);
    add(&mut g);
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    let layers = map_model::node::brushes_by_layer(h.tree_world());
    assert_eq!(layers.len(), 1);
    assert_eq!(h.tree_world().layers[layers[0].2].name, "b");
}

#[test]
fn a_brush_added_to_a_layer_removed_meanwhile_lands_on_the_default_layer() {
    // Removal and addition cross on the wire.
    let (host, guest, mut h, mut g) = layered();
    assert!(h.remove_layer(2));
    g.set_current_layer(2);
    add(&mut g);
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    assert_eq!(on_default_layer(&h), 1);
    assert_eq!(on_default_layer(&g), 1);

    // The addition arrives first, then the removal.
    let (host, guest, mut h, mut g) = layered();
    g.set_current_layer(2);
    add(&mut g);
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert!(h.remove_layer(2));
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text());
    assert_eq!(on_default_layer(&h), 1);

    // The guest's own add still undoes as one step, and the undo reaches the host.
    g.undo();
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(map_model::node::brushes_with_ids(h.tree_world()).len(), 0);
    assert_eq!(h.to_map_text(), g.to_map_text());
}

#[test]
fn a_change_too_large_for_the_session_is_refused_and_the_map_left_alone() {
    let (host, guest) = pair();
    let (host, guest) = (Rc::new(host), Rc::new(guest));
    let mut seed = Document::new(WorldBounds::default());
    add(&mut seed);
    let mut h = wired(host.clone(), seed.tree_world());
    let mut g = wired(guest.clone(), seed.tree_world());
    let before = g.to_map_text();
    crate::collab::net::TEST_EDIT_LIMIT.with(|limit| limit.set(256));
    // Through the ordinary edit path and through a drag's commit.
    g.select_all();
    g.translate_selection(Vec3d::new(16., 0., 0.));
    assert_eq!(g.to_map_text(), before, "the move was not made");
    assert!(g.undo().is_none(), "and nothing went on the undo stack");
    g.begin_transaction();
    g.translate_selection(Vec3d::new(16., 0., 0.));
    g.commit_transaction();
    assert_eq!(g.to_map_text(), before, "the drag was taken back");
    crate::collab::net::TEST_EDIT_LIMIT.with(|limit| limit.set(crate::collab::net::MAX_EDIT_FRAME));
    pump(&mut [(&host, &mut h), (&guest, &mut g)]);
    assert_eq!(h.to_map_text(), g.to_map_text(), "nobody fell out of step");
}

#[test]
fn a_select_all_move_of_the_largest_demo_map_goes_through() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../demos/lantern_lane/lantern_lane.map");
    let map = Document::open(path, WorldBounds::default()).unwrap();
    let world = map.tree_world().clone();
    let world_size = WireMessage::World(crate::collab::wire::WireWorld::from_world(&world))
        .to_json()
        .len();
    let mut doc = Document::new(WorldBounds::default());
    doc.adopt_world(world);
    let sent = Rc::new(std::cell::RefCell::new(Vec::new()));
    doc.set_on_edit(Rc::new({
        let sent = sent.clone();
        move |edit, layers| {
            let json = WireMessage::Edit(crate::collab::wire::WireEdit::from_edit_in(edit, layers))
                .to_json();
            sent.borrow_mut().push(json.len());
        }
    }));
    let before = doc.to_map_text();
    doc.select_all();
    doc.translate_selection(Vec3d::new(16., 0., 0.));
    assert_ne!(doc.to_map_text(), before);
    let edit_size = sent
        .borrow()
        .iter()
        .copied()
        .max()
        .expect("the move was sent");
    eprintln!(
        "lantern_lane: World {:.2} MiB, select-all move {:.2} MiB",
        world_size as f64 / f64::from(1 << 20),
        edit_size as f64 / f64::from(1 << 20)
    );
    assert!(world_size <= crate::collab::net::MAX_WORLD_FRAME);
    assert!(edit_size <= crate::collab::net::MAX_EDIT_FRAME);
}

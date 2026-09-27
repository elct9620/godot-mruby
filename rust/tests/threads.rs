//! gdext holds every object Godot may call from several threads in a blocking
//! cell; these are the orderings Godot's threads rely on it to keep.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use godot_cell::blocking::GdCell;

// @behavior RB-001
#[test]
fn a_mutable_call_waits_for_another_threads_call() {
    let cell = GdCell::new(0);
    let (hold, held) = mpsc::channel();
    let (bound, taken) = mpsc::channel();

    let cell = &cell;

    thread::scope(|scope| {
        let first = cell.borrow().unwrap();
        scope.spawn(move || {
            held.recv().unwrap();
            let other = cell.borrow().unwrap();
            bound.send(()).unwrap();
            thread::sleep(Duration::from_millis(50));
            drop(other);
        });
        hold.send(()).unwrap();
        taken.recv().unwrap();
        drop(first);

        let changed = cell.borrow_mut();

        assert!(changed.is_ok());
    });
}

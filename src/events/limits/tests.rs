use std::net::SocketAddr;

use super::{AcquireError, WaiterLimits};

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

#[test]
fn acquire_and_release_round_trip() {
    let limits = WaiterLimits::new(10, 2);
    let guard = limits.try_acquire(addr(1)).expect("slot available");
    assert!(limits.try_acquire(addr(2)).is_ok());
    drop(guard);

    assert!(limits.try_acquire(addr(1)).is_ok());
    assert!(limits.try_acquire(addr(2)).is_ok());
}

#[test]
fn total_limit_is_enforced() {
    let limits = WaiterLimits::new(2, 10);
    let first = limits.try_acquire(addr(1)).expect("slot available");
    let second = limits.try_acquire(addr(2)).expect("slot available");
    assert!(matches!(
        limits.try_acquire(addr(3)),
        Err(AcquireError::Total)
    ));

    drop(first);
    drop(second);
    assert!(limits.try_acquire(addr(3)).is_ok());
}

#[test]
fn per_client_limit_is_enforced() {
    let limits = WaiterLimits::new(10, 2);
    let first = limits.try_acquire(addr(1)).expect("slot available");
    let second = limits.try_acquire(addr(1)).expect("slot available");
    assert!(matches!(
        limits.try_acquire(addr(1)),
        Err(AcquireError::PerClient)
    ));
    assert!(limits.try_acquire(addr(2)).is_ok());

    drop(first);
    assert!(limits.try_acquire(addr(1)).is_ok());
    drop(second);
    assert!(limits.try_acquire(addr(1)).is_ok());
}

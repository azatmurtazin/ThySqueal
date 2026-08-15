use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcquireError {
    Total,
    PerClient,
}

pub(crate) struct WaiterLimits {
    max_total: u64,
    max_per_client: u64,
    active: AtomicU64,
    per_client: DashMap<SocketAddr, u64>,
}

impl WaiterLimits {
    pub(crate) fn new(max_total: u64, max_per_client: u64) -> Self {
        Self {
            max_total,
            max_per_client,
            active: AtomicU64::new(0),
            per_client: DashMap::new(),
        }
    }

    pub(crate) fn try_acquire(&self, addr: SocketAddr) -> Result<WaiterGuard<'_>, AcquireError> {
        if self.active.fetch_add(1, Ordering::Relaxed) >= self.max_total {
            self.active.fetch_sub(1, Ordering::Relaxed);
            return Err(AcquireError::Total);
        }
        let mut entry = self.per_client.entry(addr).or_insert(0);
        if *entry >= self.max_per_client {
            self.active.fetch_sub(1, Ordering::Relaxed);
            return Err(AcquireError::PerClient);
        }
        *entry += 1;
        Ok(WaiterGuard {
            limits: self,
            addr: Some(addr),
        })
    }

    fn release(&self, addr: Option<SocketAddr>) {
        if let Some(addr) = addr
            && let Some(mut count) = self.per_client.get_mut(&addr)
        {
            if *count <= 1 {
                drop(count);
                self.per_client.remove(&addr);
            } else {
                *count -= 1;
            }
        }
        self.active.fetch_sub(1, Ordering::Relaxed);
    }
}

pub(crate) struct WaiterGuard<'a> {
    limits: &'a WaiterLimits,
    addr: Option<SocketAddr>,
}

impl Drop for WaiterGuard<'_> {
    fn drop(&mut self) {
        self.limits.release(self.addr);
    }
}

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore};

#[derive(Clone)]
pub struct ShutdownNotify {
    notify: Arc<Notify>,
    semaphore: Arc<Semaphore>,
    waiters: Arc<AtomicU32>,
    shutting_down: Arc<AtomicBool>,
}

pub struct ShutdownGuard {
    _permits: OwnedSemaphorePermit,
    notify: Arc<Notify>,
}

impl ShutdownNotify {
    pub fn new() -> ShutdownNotify {
        ShutdownNotify {
            notify: Arc::new(Notify::const_new()),
            semaphore: Arc::new(Semaphore::const_new(0)),
            waiters: Arc::new(AtomicU32::new(0)),
            shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) -> ShutdownGuard {
        if self.shutting_down.load(Ordering::SeqCst) {
            panic!("`start` cannot be called during shutdown");
        }
        self.waiters.fetch_add(1, Ordering::SeqCst);
        // 添加一张许可证, 然后马上获取它
        self.semaphore.add_permits(1);
        // 在ShutdownGuard drop之后, 这张许可证会释放(归还)
        ShutdownGuard {
            _permits: self.semaphore.clone().try_acquire_owned().unwrap(),
            notify: self.notify.clone(),
        }
    }

    pub async fn wait_shutdown(self, timeout: Duration) {
        if self.shutting_down.load(Ordering::SeqCst) {
            panic!("double shutdown");
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            // 如果能获取到许可证, 证明已经有ShutdownGuard被drop.
            permit = self.semaphore.acquire() => {
                drop(permit);
            }
        }

        self.shutting_down.store(true, Ordering::SeqCst);
        let waiters = self.waiters.load(Ordering::SeqCst);
        // 通知全部ShutdownGuard
        for _ in 0..waiters {
            self.notify.notify_one();
        }
        tokio::time::timeout(timeout, async {
            // 等待ShutdownGuard归还许可证
            self.semaphore.acquire_many(waiters).await.unwrap().forget();
        })
        .await
        .ok();
    }
}

impl ShutdownGuard {
    // 等待关机通知
    pub async fn notified(&self) {
        self.notify.notified().await
    }
}

impl Default for ShutdownNotify {
    fn default() -> Self {
        Self::new()
    }
}

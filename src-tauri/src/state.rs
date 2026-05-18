use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::model::Config;
use crate::notifications::tracker::ChangeTracker;
use crate::polling::poller::Poller;
use crate::providers::traits::PrProvider;
use crate::providers::types::UserId;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub config: RwLock<Config>,
    pub poller: RwLock<Option<Poller>>,
    pub providers: RwLock<Vec<(Arc<dyn PrProvider>, UserId)>>,
    pub change_tracker: RwLock<ChangeTracker>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config: RwLock::new(config),
                poller: RwLock::new(None),
                providers: RwLock::new(Vec::new()),
                change_tracker: RwLock::new(ChangeTracker::new()),
            }),
        }
    }

    pub fn config(&self) -> &RwLock<Config> {
        &self.inner.config
    }

    pub fn poller(&self) -> &RwLock<Option<Poller>> {
        &self.inner.poller
    }

    pub fn providers(&self) -> &RwLock<Vec<(Arc<dyn PrProvider>, UserId)>> {
        &self.inner.providers
    }

    pub fn change_tracker(&self) -> &RwLock<ChangeTracker> {
        &self.inner.change_tracker
    }
}

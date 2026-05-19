use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::model::Config;
use crate::notifications::tracker::ChangeTracker;
use crate::polling::poller::Poller;
use crate::providers::traits::PrProvider;
use crate::providers::types::{BuildStatus, UserId};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

pub struct AppStateInner {
    pub config: RwLock<Config>,
    pub poller: RwLock<Option<Poller>>,
    pub providers: RwLock<Vec<(Arc<dyn PrProvider>, UserId)>>,
    pub change_tracker: RwLock<ChangeTracker>,
    pub build_cache: RwLock<HashMap<String, CachedBuildStatus>>,
}

pub struct CachedBuildStatus {
    pub source_commit_id: String,
    pub status: BuildStatus,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                config: RwLock::new(config),
                poller: RwLock::new(None),
                providers: RwLock::new(Vec::new()),
                change_tracker: RwLock::new(ChangeTracker::new()),
                build_cache: RwLock::new(HashMap::new()),
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

    pub fn build_cache(&self) -> &RwLock<HashMap<String, CachedBuildStatus>> {
        &self.inner.build_cache
    }
}

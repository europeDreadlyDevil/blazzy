use std::path::PathBuf;
use lru::{LruCache};
use crate::observer::Data;

pub struct Cacher {
    lru_cache: LruCache<PathBuf, Data>
}

impl Cacher {
    pub fn init() -> Self {
        Self {
            lru_cache: LruCache::unbounded()
        }
    }

    pub async fn put(&mut self, path_buf: PathBuf, data: Data) {
        self.lru_cache.put(path_buf, data);
    }

    pub async fn get(&self) -> &LruCache<PathBuf, Data> {
        &self.lru_cache
    }
}

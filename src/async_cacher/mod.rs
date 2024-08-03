use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::{mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, Mutex};
use crate::observer::Data;
use std::sync::Arc;
use lazy_static::lazy_static;

lazy_static!{
    pub static ref SharedAsyncCacher: Arc<AsyncCacher> = Arc::new(AsyncCacher::init());
}

pub struct AsyncCacher {
    tx: UnboundedSender<AsyncReq>,
    rx: Arc<Mutex<UnboundedReceiver<AsyncRes>>>,
}

impl AsyncCacher {
    pub fn init() -> Self {
        let (tx, mut rx) = unbounded_channel();
        let (tx_, rx_) = unbounded_channel();
        let rx_ = Arc::new(Mutex::new(rx_));

        let cacher = Self {
            tx: tx.clone(),
            rx: rx_.clone(),
        };

        tokio::task::spawn(async move {
            let mut map = HashMap::new();
            while let Some(action) = rx.recv().await {
                match action {
                    AsyncReq::Put(p, d) => {
                        map.insert(p, d);
                    }
                    AsyncReq::Get => {
                        tx_.clone().send(AsyncRes::Get(map.clone())).unwrap();
                    }
                    AsyncReq::Pop => {
                        let item = map.iter().last();
                        if let Some((key, value)) = item {
                            tx_.clone().send(AsyncRes::Pop(Some((key.clone(), value.clone())))).unwrap();
                            map.remove(&key.clone());
                        } else {
                            tx_.clone().send(AsyncRes::Pop(None)).unwrap();
                        }
                    }
                    AsyncReq::IsEmpty => {
                        tx_.send(AsyncRes::IsEmpty(map.is_empty())).unwrap();
                    }
                }
            }
        });

        cacher
    }

    pub fn put(&self, path_buf: PathBuf, data: Data) {
        self.tx.send(AsyncReq::Put(path_buf, data)).unwrap();
    }

    pub async fn get(&self) -> HashMap<PathBuf, Data> {
        self.tx.send(AsyncReq::Get).unwrap();
        let mut l = HashMap::new();
        let mut rx = self.rx.lock().await;
        while let Some(res) = rx.recv().await {
            if let AsyncRes::Get(lru) = res {
                l = lru;
                break;
            }
        }
        l
    }

    pub async fn pop(&self) -> Option<(PathBuf, Data)> {
        self.tx.send(AsyncReq::Pop).unwrap();
        let mut l = None;
        let mut rx = self.rx.lock().await;
        while let Some(res) = rx.recv().await {
            if let AsyncRes::Pop(lru) = res {
                l = lru;
                break;
            }
        }
        l
    }

    pub async fn is_empty(&self) -> bool {
        self.tx.send(AsyncReq::IsEmpty).unwrap();
        let mut is_empty = false;
        let mut rx = self.rx.lock().await;
        while let Some(res) = rx.recv().await {
            if let AsyncRes::IsEmpty(b) = res {
                is_empty = b;
                break;
            }
        }
        is_empty
    }
}

enum AsyncReq {
    Put(PathBuf, Data),
    Get,
    Pop,
    IsEmpty,
}

enum AsyncRes {
    Get(HashMap<PathBuf, Data>),
    IsEmpty(bool),
    Pop(Option<(PathBuf, Data)>),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::async_cacher::AsyncCacher;
    use crate::observer::{Action, Data};

    #[tokio::test]
    async fn put_and_get_test() {
        let cacher = AsyncCacher::init().await;
        cacher.put(PathBuf::new(), Data::new(Action::Created, None));
        for (key, value) in cacher.get().await {
            assert_eq!((key, value), (PathBuf::new(), Data::new(Action::Created, None)));
        }
    }

    #[tokio::test]
    async fn pop_test() {
        let cacher = AsyncCacher::init().await;
        cacher.put(PathBuf::new(), Data::new(Action::Created, None));
        let item = cacher.pop().await;
        assert_eq!(item, Some((PathBuf::new(), Data::new(Action::Created, None))));
        assert_eq!(cacher.is_empty().await, true);
    }
}

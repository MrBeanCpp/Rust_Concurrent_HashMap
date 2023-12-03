//! Thread-safe key/value cache.

use std::collections::hash_map::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Debug, Default)]
pub struct Cache<K, V> {
    // todo! This is an example cache type. Build your own cache type that satisfies the
    // specification for `get_or_insert_with`.
    inner: RwLock<HashMap<K, V>>,
    locker: Mutex<HashMap<K, Arc<Mutex<()>>>>
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    // 得到 `key` 对应的值，如果不存在则调用 `f` 函数得到值并插入到缓存中
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        if let Some(v) = self.inner.read().unwrap().get(&key).cloned() { //获取读锁，检测Key是否存在
            return v;
        } //自动释放锁
        let lock_k = { //获取每个Key对应的小锁，并行不同Key，提高并发度
            let mut lock_map = self.locker.lock().unwrap(); //锁定存放小锁的HashMap（这里不用读写锁为了降低编程复杂度）
            if let Some(v) = lock_map.get(&key) { //检测该key对应小锁是否存在
                Arc::clone(v)
            } else {
                let lock = Arc::new(Mutex::new(()));
                lock_map.insert(key.clone(), Arc::clone(&lock)); //不存在则插入小锁
                Arc::clone(&lock) //并返回该锁的指针
            }
        }; //自动释放锁
        let _lock_k = lock_k.lock().unwrap(); //尝试获取该key对应的小锁；这样不同的key可以并发，也防止相同key同时写入（f(key)只能被调用一次）
        if let Some(v) = self.inner.read().unwrap().get(&key).cloned() { //竞争成功后 还需要再次read（读锁），防止其他线程已经写入
            return v;
        } //释放锁（细粒度加锁提高并发度）
        let v = f(key.clone()); //耗时操作 需要并发
        let mut lock_map = self.inner.write().unwrap(); //获取写锁
        lock_map.insert(key.clone(), v.clone()); //写入
        return v; //返回并释放写锁
    }
}

#[cfg(test)]
mod test {
    use super::Cache;
    use crossbeam_channel::bounded;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Barrier;
    use std::thread::scope;
    use std::time::Duration;

    const NUM_THREADS: usize = 8;
    const NUM_KEYS: usize = 128;

    #[test]
    fn cache_no_duplicate_sequential() {
        let cache = Cache::default();
        let _ = cache.get_or_insert_with(1, |_| 1);
        let _ = cache.get_or_insert_with(2, |_| 2);
        let _ = cache.get_or_insert_with(3, |_| 3);
        assert_eq!(cache.get_or_insert_with(1, |_| panic!()), 1);
        assert_eq!(cache.get_or_insert_with(2, |_| panic!()), 2);
        assert_eq!(cache.get_or_insert_with(3, |_| panic!()), 3);
    }

    #[test]
    fn cache_no_duplicate_concurrent() {
        for _ in 0..8 {
            let cache = Cache::default();
            let barrier = Barrier::new(NUM_THREADS);
            // Count the number of times the computation is run.
            let num_compute = AtomicUsize::new(0);
            scope(|s| {
                for _ in 0..NUM_THREADS {
                    let _unused = s.spawn(|| {
                        let _ = barrier.wait();
                        for key in 0..NUM_KEYS {
                            let _ = cache.get_or_insert_with(key, |k| {
                                let _ = num_compute.fetch_add(1, Ordering::Relaxed);
                                k
                            });
                        }
                    });
                }
            });
            assert_eq!(num_compute.load(Ordering::Relaxed), NUM_KEYS);
        }
    }

    #[test]
    fn cache_no_block_disjoint() {
        let cache = &Cache::default();

        scope(|s| {
            // T1 blocks while inserting 1.
            let (t1_quit_sender, t1_quit_receiver) = bounded(0);
            let _unused = s.spawn(move || {
                let _ = cache.get_or_insert_with(1, |k| {
                    // block T1
                    t1_quit_receiver.recv().unwrap();
                    k
                });
            });

            // T2 must not be blocked by T1 when inserting 2.
            let (t2_done_sender, t2_done_receiver) = bounded(0);
            let _unused = s.spawn(move || {
                let _ = cache.get_or_insert_with(2, |k| k);
                t2_done_sender.send(()).unwrap();
            });

            // If T2 is blocked, then this will time out.
            t2_done_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("Inserting a different key should not block");

            // clean up
            t1_quit_sender.send(()).unwrap();
        });
    }

    #[test]
    fn cache_no_reader_block() {
        let cache = &Cache::default();

        scope(|s| {
            let (t1_quit_sender, t1_quit_receiver) = bounded(0);
            let (t3_done_sender, t3_done_receiver) = bounded(0);

            // T1 blocks while inserting 1.
            let _unused = s.spawn(move || {
                let _ = cache.get_or_insert_with(1, |k| {
                    // T2 is blocked by T1 when reading 1
                    let _unused = s.spawn(move || cache.get_or_insert_with(1, |_| panic!()));

                    // T3 should not be blocked when inserting 3.
                    let _unused = s.spawn(move || {
                        let _ = cache.get_or_insert_with(3, |k| k);
                        t3_done_sender.send(()).unwrap();
                    });

                    // block T1
                    t1_quit_receiver.recv().unwrap();
                    k
                });
            });

            // If T3 is blocked, then this will time out.
            t3_done_receiver
                .recv_timeout(Duration::from_secs(3))
                .expect("Inserting a different key should not block");

            // clean up
            t1_quit_sender.send(()).unwrap();
        });
    }
}

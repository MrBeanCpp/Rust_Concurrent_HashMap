# Rust_Concurrent_HashMap
Rust实验：并发安全的缓存设计；设计一个并发安全的 key-value 缓存，同时保证一定的性能  
> 缓存是一种常见的设计模式，它可以将一些计算结果缓存起来，以便下次使用时可以直接从缓存中获取，而不需要重新计算。本次实验，你需要设计一个并发安全的 key-value 缓存，同时保证一定的性能。

```cpp
#[derive(Debug, Default)]
pub struct Cache<K, V> {
    // todo! 这只是一个示例，你需要修改它的类型来满足题目要求
    inner: Mutex<HashMap<K, V>>
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// 得到 `key` 对应的值，如果不存在则调用 `f` 函数得到值并插入到缓存中
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        todo!()
    }
}
```
你需要设计 Cache 结构体里面 inner 的类型以及实现其 get_or_insert_with 方法，使得该函数满足以下要求：

- 一方面，对于不同的 key，get_or_insert_with 函数的调用应该是并发的。例如，如果一个线程调用 `get_or_insert_with(key1, f1)`，另一个线程调用 `get_or_insert_with(key2, f2)`，且`(key1 != key2)`，那么 `f1` 和 `f2` 应该是并发执行的。
- 另一方面，我们认为 `f` 函数的调用是十分耗费资源的，所以你需要保证 f 函数对于某一个 key 只会被调用一次。特别的，即使有多个线程同时调用 `get_or_insert_with` 函数，`f` 函数也只会被调用一次。

## 实验指导
参考解答大约只需要 20 行代码，且恰好使用了所有在 lib.rs 中引入的所有同步原语

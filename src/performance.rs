use crate::domain::*;
use crate::error::{ClaudelyticsError, Result};
use crate::processing::{RawUsageRecord, RecordConverter, RecordValidator};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// メモリ効率的なデータストリーミング処理
#[allow(dead_code)]
pub struct StreamProcessor {
    validator: RecordValidator,
    converter: RecordConverter,
    chunk_size: usize,
}

#[allow(dead_code)]
impl StreamProcessor {
    pub fn new(validator: RecordValidator, converter: RecordConverter, chunk_size: usize) -> Self {
        Self {
            validator,
            converter,
            chunk_size,
        }
    }

    /// ファイルをストリーミング処理し、メモリ使用量を制限
    pub fn process_file_stream<F>(
        &self,
        file_path: &Path,
        session_id: SessionId,
        mut processor: F,
    ) -> Result<()>
    where
        F: FnMut(Vec<UsageEvent>) -> Result<()>,
    {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut chunk = Vec::with_capacity(self.chunk_size);

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<RawUsageRecord>(&line) {
                Ok(record) if self.validator.is_valid(&record) => {
                    if let Some(event) =
                        self.converter.convert_to_event(&record, session_id.clone())
                    {
                        chunk.push(event);

                        if chunk.len() >= self.chunk_size {
                            processor(std::mem::take(&mut chunk))?;
                            chunk = Vec::with_capacity(self.chunk_size);
                        }
                    }
                }
                _ => continue, // 無効なレコードはスキップ
            }
        }

        // 残りのチャンクを処理
        if !chunk.is_empty() {
            processor(chunk)?;
        }

        Ok(())
    }
}

/// LRU キャッシュシステム
#[allow(dead_code)]
pub struct LruCache<K, V> {
    data: HashMap<K, (V, Instant)>,
    capacity: usize,
    ttl: Duration,
}

#[allow(dead_code)]
impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        Self {
            data: HashMap::with_capacity(capacity),
            capacity,
            ttl,
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        let now = Instant::now();

        if let Some((value, timestamp)) = self.data.get(key) {
            if now.duration_since(*timestamp) < self.ttl {
                Some(value.clone())
            } else {
                self.data.remove(key);
                None
            }
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        let now = Instant::now();

        // 容量制限チェック
        if self.data.len() >= self.capacity && !self.data.contains_key(&key) {
            self.evict_old_entries(now);
        }

        self.data.insert(key, (value, now));
    }

    fn evict_old_entries(&mut self, now: Instant) {
        self.data
            .retain(|_, (_, timestamp)| now.duration_since(*timestamp) < self.ttl);

        // まだ容量オーバーの場合、最も古いエントリを削除
        if self.data.len() >= self.capacity {
            if let Some(oldest_key) = self
                .data
                .iter()
                .min_by_key(|(_, (_, timestamp))| *timestamp)
                .map(|(k, _)| k.clone())
            {
                self.data.remove(&oldest_key);
            }
        }
    }
}

/// オブジェクトプールによるメモリ効率化
#[allow(dead_code)]
pub struct ObjectPool<T> {
    objects: Arc<Mutex<Vec<T>>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
}

#[allow(dead_code)]
impl<T> ObjectPool<T>
where
    T: Default + Send + 'static,
{
    pub fn new<F>(capacity: usize, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let mut objects = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            objects.push(factory());
        }

        Self {
            objects: Arc::new(Mutex::new(objects)),
            factory: Box::new(factory),
        }
    }

    pub fn get(&self) -> Result<PooledObject<T>> {
        let obj = {
            let mut objects = self
                .objects
                .lock()
                .map_err(|_| ClaudelyticsError::other("Failed to acquire object pool lock"))?;
            objects.pop().unwrap_or_else(|| (self.factory)())
        };

        Ok(PooledObject {
            object: Some(obj),
            pool: Arc::clone(&self.objects),
        })
    }
}

/// プールから借りたオブジェクト
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
}

#[allow(dead_code)]
impl<T> PooledObject<T> {
    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.object.as_mut()
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object
            .as_ref()
            .expect("PooledObject should always contain an object")
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object
            .as_mut()
            .expect("PooledObject should always contain an object")
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.object.take() {
            if let Ok(mut objects) = self.pool.lock() {
                objects.push(obj);
            }
            // If we can't acquire the lock, we simply let the object be dropped
            // This is safer than panicking in a destructor
        }
    }
}

/// 並列処理の最適化された実装
#[allow(dead_code)]
pub struct OptimizedParallelProcessor {
    thread_pool: rayon::ThreadPool,
    cache: Arc<Mutex<LruCache<String, UsageMetrics>>>,
}

#[allow(dead_code)]
impl OptimizedParallelProcessor {
    pub fn new(num_threads: usize, cache_capacity: usize) -> Result<Self> {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|e| {
                ClaudelyticsError::other(&format!("Failed to create thread pool: {}", e))
            })?;

        let cache = Arc::new(Mutex::new(LruCache::new(
            cache_capacity,
            Duration::from_secs(300), // 5分間キャッシュ
        )));

        Ok(Self { thread_pool, cache })
    }

    /// 複数ファイルを最適化された並列処理で処理
    pub fn process_files_optimized<P>(
        &self,
        file_paths: Vec<P>,
        processor: fn(&Path) -> Result<Vec<UsageEvent>>,
    ) -> Result<Vec<UsageEvent>>
    where
        P: AsRef<Path> + Send + Sync,
    {
        let results: Result<Vec<_>> = self.thread_pool.install(|| {
            file_paths
                .par_iter()
                .map(|path| {
                    let path_ref = path.as_ref();

                    // キャッシュチェック
                    if let Some(cached) = self.get_cached_result(path_ref) {
                        return Ok(cached);
                    }

                    // ファイル処理
                    let events = processor(path_ref)?;

                    // 結果をキャッシュ
                    self.cache_result(path_ref, &events);

                    Ok(events)
                })
                .collect()
        });

        let all_results = results?;
        Ok(all_results.into_iter().flatten().collect())
    }

    fn get_cached_result(&self, _path: &Path) -> Option<Vec<UsageEvent>> {
        // 実装簡素化のため、キャッシュは省略
        // 実際の実装では、ファイルの更新時刻をチェックしてキャッシュの有効性を判断
        None
    }

    fn cache_result(&self, _path: &Path, _events: &[UsageEvent]) {
        // 実装簡素化のため、キャッシュは省略
    }
}

/// メモリ使用量モニタリング
pub struct MemoryMonitor {
    peak_usage: usize,
    current_usage: usize,
    limit: Option<usize>,
}

#[allow(dead_code)]
impl MemoryMonitor {
    pub fn new(limit_mb: Option<usize>) -> Self {
        Self {
            peak_usage: 0,
            current_usage: 0,
            limit: limit_mb.map(|mb| mb * 1024 * 1024),
        }
    }

    pub fn track_allocation(&mut self, size: usize) -> Result<()> {
        self.current_usage += size;
        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }

        if let Some(limit) = self.limit {
            if self.current_usage > limit {
                return Err(ClaudelyticsError::other(&format!(
                    "Memory limit exceeded: {} MB",
                    limit / 1024 / 1024
                )));
            }
        }

        Ok(())
    }

    pub fn track_deallocation(&mut self, size: usize) {
        self.current_usage = self.current_usage.saturating_sub(size);
    }

    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            current_usage_mb: self.current_usage / 1024 / 1024,
            peak_usage_mb: self.peak_usage / 1024 / 1024,
            limit_mb: self.limit.map(|l| l / 1024 / 1024),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MemoryStats {
    pub current_usage_mb: usize,
    pub peak_usage_mb: usize,
    pub limit_mb: Option<usize>,
}

/// 遅延評価によるデータ処理最適化
type ProcessorFn<T> = Box<dyn Fn(&[T]) -> Vec<T> + Send + Sync>;

#[allow(dead_code)]
pub struct LazyDataProcessor<T> {
    data: Vec<T>,
    processed: bool,
    processor: ProcessorFn<T>,
}

#[allow(dead_code)]
impl<T> LazyDataProcessor<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new<F>(data: Vec<T>, processor: F) -> Self
    where
        F: Fn(&[T]) -> Vec<T> + Send + Sync + 'static,
    {
        Self {
            data,
            processed: false,
            processor: Box::new(processor),
        }
    }

    pub fn get_processed(&mut self) -> &[T] {
        if !self.processed {
            self.data = (self.processor)(&self.data);
            self.processed = true;
        }
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// パフォーマンス測定ユーティリティ
#[allow(dead_code)]
pub struct PerformanceProfiler {
    start_time: Instant,
    checkpoints: Vec<(String, Instant)>,
}

#[allow(dead_code)]
impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            checkpoints: Vec::new(),
        }
    }

    pub fn checkpoint(&mut self, name: impl Into<String>) {
        self.checkpoints.push((name.into(), Instant::now()));
    }

    pub fn finish(self) -> PerformanceReport {
        let total_duration = self.start_time.elapsed();
        let mut sections = Vec::new();

        let mut prev_time = self.start_time;
        for (name, timestamp) in self.checkpoints {
            let duration = timestamp.duration_since(prev_time);
            sections.push(PerformanceSection {
                name,
                duration,
                percentage: (duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0,
            });
            prev_time = timestamp;
        }

        PerformanceReport {
            total_duration,
            sections,
        }
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct PerformanceReport {
    pub total_duration: Duration,
    pub sections: Vec<PerformanceSection>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PerformanceSection {
    pub name: String,
    pub duration: Duration,
    pub percentage: f64,
}

#[allow(dead_code)]
impl PerformanceReport {
    pub fn print_summary(&self) {
        println!("Performance Report:");
        println!("Total Duration: {:?}", self.total_duration);
        println!("Sections:");
        for section in &self.sections {
            println!(
                "  {}: {:?} ({:.1}%)",
                section.name, section.duration, section.percentage
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(2, Duration::from_secs(60));

        cache.insert("key1", "value1");
        cache.insert("key2", "value2");

        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), Some("value2"));

        // 容量超過でエビクション
        cache.insert("key3", "value3");
        assert!(cache.get(&"key1").is_none() || cache.get(&"key2").is_none());
    }

    #[test]
    fn test_object_pool() {
        let pool = ObjectPool::new(2, Vec::<i32>::new);

        let mut obj1 = pool.get().expect("Should get object from pool");
        obj1.push(1);
        assert_eq!(obj1.len(), 1);

        drop(obj1);

        let _obj2 = pool.get().expect("Should get object from pool");
        // プールから再利用されるが、内容はクリアされていない可能性がある
        // 実際の使用では、オブジェクトの初期化が必要
    }

    #[test]
    fn test_object_pool_drop_safety() {
        // Test that dropping a PooledObject doesn't panic even if mutex is problematic
        let pool = ObjectPool::new(1, Vec::<i32>::new);

        let obj = pool.get().expect("Should get object from pool");
        // Object will be returned to pool on drop
        drop(obj);

        // Get the object again to verify pool still works
        let obj2 = pool.get().expect("Should get object from pool after drop");
        drop(obj2);

        // Test passed if we got here without panicking
    }

    #[test]
    fn test_memory_monitor() {
        let mut monitor = MemoryMonitor::new(Some(1)); // 1MB制限

        assert!(monitor.track_allocation(500 * 1024).is_ok()); // 500KB
        assert!(monitor.track_allocation(600 * 1024).is_err()); // 制限超過

        let stats = monitor.get_stats();
        assert!(stats.current_usage_mb > 0);
    }

    #[test]
    fn test_performance_profiler() {
        let mut profiler = PerformanceProfiler::new();

        std::thread::sleep(Duration::from_millis(10));
        profiler.checkpoint("step1");

        std::thread::sleep(Duration::from_millis(20));
        profiler.checkpoint("step2");

        let report = profiler.finish();
        assert!(report.total_duration.as_millis() >= 30);
        assert_eq!(report.sections.len(), 2);
    }
}

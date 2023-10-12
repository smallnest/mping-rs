use std::cmp::Ordering;
use std::{
    collections::BinaryHeap,
    collections::HashMap,
    sync::{Mutex, RwLock},
};

/// Bucket is used to store the ping results of one second for all targets.
pub struct Bucket {
    // key is the timestamp of the bucket in seconds.
    pub key: u128,
    // value is the ping results of all targets in the bucket.
    pub value: RwLock<HashMap<String, Result>>,
}

impl Clone for Bucket {
    fn clone(&self) -> Self {
        let value = self.value.read().unwrap().clone();

        Bucket {
            key: self.key,
            value: RwLock::new(value),
        }
    }
}

impl Ord for Bucket {
    fn cmp(&self, other: &Bucket) -> Ordering {
        // 提取关键字进行比较
        self.key.cmp(&other.key).reverse()
    }
}

impl PartialOrd for Bucket {
    fn partial_cmp(&self, other: &Bucket) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Bucket {}

impl PartialEq for Bucket {
    fn eq(&self, other: &Bucket) -> bool {
        self.key == other.key
    }
}

impl Bucket {
    /// Create a new bucket.
    pub fn new(key: u128) -> Self {
        Bucket {
            key,
            value: RwLock::new(HashMap::new()),
        }
    }

    /// Add a ping result to the bucket.
    pub fn add(&self, result: Result) {
        let mut map = self.value.write().unwrap();
        let key = format!("{}-{}", &result.target, &result.seq);
        map.insert(key, result);
    }

    /// Add a ping reply to the bucket.
    pub fn add_reply(&self, mut result: Result) {
        let mut map = self.value.write().unwrap();

        let key = format!("{}-{}", result.target, &result.seq);
        if let Some(req) = map.get(&key) {
            result.txts = req.txts;
            result.calc_latency();
        }
        map.insert(key, result.clone());
    }

    /// Update the txts (software/hardware timestamp) of the ping result after send.
    pub fn update_txts(&self, target: String, seq: u16, txts: u128) {
        let mut map = self.value.write().unwrap();

        let key = format!("{}-{}", target, seq);
        if let Some(result) = map.get_mut(&key) {
            result.txts = txts;
        }
    }

    /// Get the ping result of all targets.
    pub fn values(&self) -> Vec<Result> {
        let map = self.value.read().unwrap();
        map.values().cloned().collect()
    }
}

/// Buckets is used to store all unprocessed buckets.
#[derive(Default)]
pub struct Buckets {
    // buckets is a priority queue, the top of the queue is the bucket with the smallest key.
    pub buckets: Mutex<BinaryHeap<Bucket>>,
    // map is used to quickly find the bucket by key.
    pub map: Mutex<HashMap<u128, Bucket>>,
}

impl Buckets {
    /// Create a new buckets.
    pub fn new() -> Buckets {
        Buckets {
            buckets: Mutex::new(BinaryHeap::new()),
            map: Mutex::new(HashMap::new()),
        }
    }

    /// Add a ping result to the buckets. The key is the timestamp of the bucket in seconds.
    pub fn add(&self, key: u128, value: Result) {
        let mut map = self.map.lock().unwrap();
        map.entry(key).or_insert_with(|| {
            let bucket = Bucket {
                key,
                value: RwLock::new(HashMap::new()),
            };
            self.buckets.lock().unwrap().push(bucket.clone());
            bucket
        });

        let bucket = map.get(&key).unwrap();
        bucket.add(value);
    }

    /// Add a ping reply to the buckets. The key is the timestamp of the bucket in seconds.
    pub fn add_reply(&self, key: u128, result: Result) {
        let mut map = self.map.lock().unwrap();

        map.entry(key).or_insert_with(|| {
            self.buckets.lock().unwrap().push(Bucket::new(key));
            Bucket::new(key)
        });

        let bucket = map.get(&key).unwrap();
        bucket.add_reply(result);
    }

    /// Update the txts (software/hardware timestamp) of the ping result after send.
    /// The key is the timestamp of the bucket in seconds.
    pub fn update_txts(&self, key: u128, target: String, seq: u16, txts: u128) {
        let map = self.map.lock().unwrap();

        if let Some(bucket) = map.get(&key) {
            bucket.update_txts(target, seq, txts);
        }
    }

    /// pop the bucket with the smallest key.
    pub fn pop(&self) -> Option<Bucket> {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.pop()?;
        let bucket = self.map.lock().unwrap().remove(&bucket.key).unwrap();
        Some(bucket)
    }

    /// Get(but not pop) the bucket with the smallest key.
    pub fn last(&self) -> Option<Bucket> {
        let buckets = self.buckets.lock().unwrap();
        buckets.peek().cloned()
    }
}

/// Result is used to store one ping result of one target.
/// The result is indentified by the target and the sequence number.
#[derive(Default, Clone, Debug)]
pub struct Result {
    /// The timestamp when the ping request is sent.
    pub txts: u128,
    /// The timestamp when the ping reply is received.
    pub rxts: u128,
    /// The sequence number of the ping request.
    pub seq: u16,
    /// The target of the ping result.
    pub target: String,
    /// The latency of the ping result.
    pub latency: u128,
    /// Received is true if the ping reply is received.
    pub received: bool,
    /// Bitflip is true if the ping reply is received but the data is corrupted.
    pub bitflip: bool,
}

impl Result {
    /// Create a new ping result.
    pub fn new(txts: u128, target: &str, seq: u16) -> Self {
        Result {
            txts,
            target: target.to_string(),
            seq,
            ..Default::default()
        }
    }

    /// Calculate the latency of the ping result.
    pub fn calc_latency(&mut self) {
        self.latency = self.rxts - self.txts;
    }
}

/// TargetResult is used to store the ping stat result of one target.
#[derive(Default, Clone, Debug)]
pub struct TargetResult {
    /// The target of the ping result.
    pub target: String,
    /// The loss rate of the ping result.
    pub loss_rate: f64,
    /// The average latency of the ping result.
    pub latency: u128,
    /// Theloss count of the ping result.
    pub loss: u32,
    /// The received count of the ping result.
    pub received: u32,
    /// The bitflip count of the ping result.
    pub bitflip_count: u32,
}

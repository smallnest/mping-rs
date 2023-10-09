use std::cmp::Ordering;
use std::{
    collections::BinaryHeap,
    collections::HashMap,
    sync::{Mutex, RwLock},
};

pub struct Bucket {
    pub key: u128,
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
    pub fn new(key: u128) -> Self {
        Bucket {
            key,
            value: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, result: Result) {
        let mut map = self.value.write().unwrap();
        let key = format!("{}-{}", &result.target, &result.seq);
        map.insert(key, result);
    }

    pub fn add_reply(&self, mut result: Result) {
        let mut map = self.value.write().unwrap();

        let key = format!("{}-{}", result.target, &result.seq);
        if let Some(req) = map.get(&key) {
            result.txts = req.txts;
            result.calc_latency();
        }
        map.insert(key, result.clone());
    }

    pub fn update_txts(&self, target: String, seq: u16, txts: u128) {
        let mut map = self.value.write().unwrap();

        let key = format!("{}-{}", target, seq);
        if let Some(result) = map.get_mut(&key) {
            result.txts = txts;
        }
    }

    pub fn values(&self) -> Vec<Result> {
        let map = self.value.read().unwrap();
        map.values().cloned().collect()
    }
}

#[derive(Default)]
pub struct Buckets {
    pub buckets: Mutex<BinaryHeap<Bucket>>,
    pub map: Mutex<HashMap<u128, Bucket>>,
}

impl Buckets {
    pub fn new() -> Buckets {
        Buckets {
            buckets: Mutex::new(BinaryHeap::new()),
            map: Mutex::new(HashMap::new()),
        }
    }

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

    pub fn add_reply(&self, key: u128, result: Result) {
        let mut map = self.map.lock().unwrap();

        map.entry(key).or_insert_with(|| {
            self.buckets.lock().unwrap().push(Bucket::new(key));
            Bucket::new(key)
        });

        let bucket = map.get(&key).unwrap();
        bucket.add_reply(result);
    }


    pub fn update_txts(&self,  key: u128, target: String, seq: u16, txts: u128) {
        let map = self.map.lock().unwrap();

        if let Some(bucket) = map.get(&key) {
            bucket.update_txts(target, seq, txts);
        }
    }

    pub fn pop(&self) -> Option<Bucket> {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets.pop()?;
        let bucket = self.map.lock().unwrap().remove(&bucket.key).unwrap();
        Some(bucket)
    }

    pub fn last(&self) -> Option<Bucket> {
        let buckets = self.buckets.lock().unwrap();
        buckets.peek().cloned()
    }
}

#[derive(Default, Clone, Debug)]
pub struct Result {
    pub txts: u128,
    pub rxts: u128,
    pub seq: u16,
    pub target: String,
    pub latency: u128,
    pub received: bool,
    pub bitflip: bool,
}

impl Result {
    // 计算latency
    pub fn calc_latency(&mut self) {
        self.latency = self.rxts - self.txts;
    }

    pub fn new(txts: u128, target: &str, seq: u16) -> Self {
        Result {
            txts,
            target: target.to_string(),
            seq,
            ..Default::default()
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct TargetResult {
    pub latency: u128,
    pub loss: u32,
    pub received: u32,
    pub bitflip_count: u32,
}

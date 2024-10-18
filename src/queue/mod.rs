/*
Очередь. Потокобезопасный синглтон. Несколько писателей ставят в очередь,
один читатель принимает сообщения.
 */

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

pub struct TwoWayQueue {
    data: Mutex<VecDeque<String>>,
    condvar: Condvar,
}

impl TwoWayQueue {
    fn new() -> Self {
        Self {
            data: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
        }
    }

    pub fn push(&self, value: String) {
        let mut queue = self.data.lock().unwrap();
        queue.push_front(value);
        self.condvar.notify_one();
    }

    pub fn pop(&self) -> Option<String> {
        let mut queue = self.data.lock().unwrap();
        while queue.is_empty() {
            queue = self.condvar.wait(queue).unwrap();
        }
        queue.pop_back()
    }
}

pub type SharedQueue = Arc<TwoWayQueue>;

pub static INCOMING_QUEUE: Lazy<SharedQueue> = Lazy::new(|| Arc::new(TwoWayQueue::new()));

pub static OUTCOMING_QUEUE: Lazy<SharedQueue> = Lazy::new(|| Arc::new(TwoWayQueue::new()));

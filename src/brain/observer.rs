use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub type Observer = Box<dyn Fn(&String, &String, &String) + Send + Sync>;

pub struct Observable {
    observers: Arc<Mutex<Vec<Observer>>>,
    sender: mpsc::Sender<(String, String, String)>,
}

impl Observable {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let observers: Arc<Mutex<Vec<Observer>>> = Arc::new(Mutex::new(Vec::new()));

        thread::spawn({
            let observers = Arc::clone(&observers);
            move || {
                for (symbol, price, volume) in receiver {
                    let observers = observers.lock().unwrap();
                    for observer in observers.iter() {
                        observer(&symbol, &price, &volume);
                    }
                }
            }
        });

        Observable { observers, sender }
    }

    pub fn add_observer(&mut self, observer: Observer) {
        let mut observers = self.observers.lock().unwrap();
        observers.push(observer);
    }

    pub fn notify_observers(&self, symbol: String, price: String, volume: String) {
        self.sender.send((symbol, price, volume)).unwrap();
    }
}

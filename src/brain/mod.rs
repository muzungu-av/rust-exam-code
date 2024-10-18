pub mod graph;
pub mod observer;
pub mod triangle;
use crate::brain::observer::Observable;
use crate::uds_write::{uds_connect, uds_write_to};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::Local;
use crossbeam::queue::SegQueue;
use dashmap::DashMap;
use petgraph::graph::DiGraph;
use rand::Rng;
use std::fmt::Formatter;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};
use std::{iter, task};
use tokio::net::UnixStream;
use triangle::TriangleKey;

#[derive(Clone, Debug)]
struct DataStorage {
    map: DashMap<String, (String, String)>, // ключ - symbol, значение - (price, volume)
}

impl DataStorage {
    fn new() -> Self {
        DataStorage {
            map: DashMap::new(),
        }
    }

    fn insert(&self, symbol: String, price: String, volume: String) {
        // Вставляем символ с данными о цене и объеме
        self.map.insert(symbol, (price, volume));
    }

    fn count(&self) -> usize {
        return self.map.len();
    }

    // fn display(&self) {
    //     for r in self.map.iter() {
    //         println!("DataStorage  Symbol: {}, Price: {}", r.key(), r.value().0);
    //     }
    // }
}

type SymbDir = (String, String);
type SymbolRefTriangles = DashMap<String, HashMap<TriangleKey, Vec<SymbDir>>>;

lazy_static::lazy_static! {
    static ref PRICE_STORAGE: DataStorage = DataStorage::new();
    static ref COUNT: AtomicUsize = AtomicUsize::new(0);
    static ref REGULAR_MODE: AtomicBool = AtomicBool::new(false);
    static ref SRT: SymbolRefTriangles = DashMap::new();
    static ref RATE: AtomicU64 = AtomicU64::new(1.0f64.to_bits());
    static ref EARN_QUEUE: Arc<SegQueue<EarnSortedData>> = Arc::new(SegQueue::new());
}

#[derive(Debug)]
struct EarnSortedData {
    triangle_key: TriangleKey,
    final_amount: BigDecimal,
    earn: BigDecimal,
}

impl PartialEq for EarnSortedData {
    fn eq(&self, other: &Self) -> bool {
        self.earn == other.earn
    }
}

impl Eq for EarnSortedData {}

impl PartialOrd for EarnSortedData {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.earn.partial_cmp(&self.earn) // Сортировка по убыванию earn
    }
}

impl Ord for EarnSortedData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn build_immutable_storage(
    triangles: &HashMap<TriangleKey, Vec<(String, String)>>,
    unique_values: &HashSet<String>,
) {
    for value in unique_values {
        let mut matching_triangles = HashMap::new();
        for (triangle_key, triangle_values) in triangles {
            if triangle_key.a == *value || triangle_key.b == *value || triangle_key.c == *value {
                matching_triangles.insert(triangle_key.clone(), triangle_values.clone());
            }
        }

        if !matching_triangles.is_empty() {
            SRT.insert(value.clone(), matching_triangles);
        }
    }
}

pub fn initialize_observers(
    observable: Arc<Mutex<Observable>>,
    count: usize,
    triangles: &HashMap<TriangleKey, Vec<(String, String)>>,
    rate: f64,
) {
    COUNT.store(count, Ordering::SeqCst);
    RATE.store(rate.to_bits(), Ordering::SeqCst);
    let _rate = f64::from_bits(RATE.load(Ordering::SeqCst));
    let rate_bd = BigDecimal::from_f64(_rate).unwrap_or(BigDecimal::from(100));

    let mut unique_symbols: HashSet<String> = HashSet::new();
    for key in triangles.keys() {
        unique_symbols.insert(key.a.clone());
        unique_symbols.insert(key.b.clone());
        unique_symbols.insert(key.c.clone());
    }

    build_immutable_storage(&triangles, &unique_symbols);

    // Создаем `UnixStream` внутри функции
    let socket_path = "/tmp/arm_arbitr_socket";
    let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap()); // Оборачиваем в Arc

    let uid = generate_random_id(4);
    // Получаем поток асинхронно
    let runtime_clone = Arc::clone(&runtime);
    runtime.spawn(async move {
        match uds_connect(socket_path).await {
            Ok(stream) => {
                let stream = Arc::new(Mutex::new(stream)); // Оборачиваем в Arc<Mutex<UnixStream>>

                let stream_clone = Arc::clone(&stream);
                observable
                    .lock()
                    .unwrap()
                    .add_observer(Box::new(move |symbol, price, volume| {
                        PRICE_STORAGE.insert(symbol.clone(), price.clone(), volume.clone());
                        if !REGULAR_MODE.load(Ordering::SeqCst) {
                            let dsc = PRICE_STORAGE.count();
                            let c = COUNT.load(Ordering::SeqCst);
                            if dsc < c {
                                println!("наполнение осталось {}", c - dsc);
                                return;
                            } else {
                                REGULAR_MODE.store(true, Ordering::SeqCst);
                                println!("*** начало работы ***");
                            }
                        } else {
                            // Получаем доступ к потоку через Mutex
                            let mut stream = stream_clone.lock().unwrap();
                            react_to_update(symbol, &rate_bd, &mut *stream, &runtime_clone, &uid);
                        }
                    }));
            }
            Err(e) => {
                eprintln!("Ошибка подключения: {:?}", e);
            }
        }
    });
}

fn react_to_update(
    symbol: &String,
    rate: &BigDecimal,
    stream: &mut UnixStream,
    runtime: &tokio::runtime::Runtime,
    uid: &String,
) {
    if let Some(triangles) = SRT.get(symbol) {
        let mut unique_symbols: HashSet<&String> = HashSet::new();

        for (triangle_key, _) in triangles.iter() {
            unique_symbols.insert(&triangle_key.a);
            unique_symbols.insert(&triangle_key.b);
            unique_symbols.insert(&triangle_key.c);
        }
        let mut symbol_price_map: HashMap<String, (String, String)> = HashMap::new();
        for symbol in unique_symbols {
            if let Some(price_volume) = PRICE_STORAGE.map.get(symbol) {
                symbol_price_map.insert(symbol.clone(), price_volume.value().clone());
            } else {
                println!("No data found for symbol: {}", symbol);
            }
        }

        for (triangle_key, triangle) in triangles.iter() {
            if let Some((final_amount, earn)) =
                calculate_triangle(&symbol_price_map, triangle_key, triangle)
            {
                if earn >= *rate {
                    // очередь
                    EARN_QUEUE.push(EarnSortedData {
                        triangle_key: (*triangle_key).clone(),
                        final_amount: final_amount,
                        earn: earn,
                    });
                }
            }
        }
        //для сортировки
        let mut data_vec: Vec<EarnSortedData> = Vec::new();
        while let Some(data) = EARN_QUEUE.pop() {
            data_vec.push(data);
        }
        // Сортировка по earn
        data_vec.sort_by(|a, b| b.earn.partial_cmp(&a.earn).unwrap());

        if let Some(maxdata) = data_vec.first() {
            let current_time = Local::now();
            let formatted_time = current_time.format("%H:%M:%S%.6f");

            let msg_to_arm = format!(
                "{}  {} MAX -> {:?}, Final Amount: {}, Earn: {}\n",
                uid,
                formatted_time,
                maxdata.triangle_key,
                maxdata.final_amount.with_scale(6),
                maxdata.earn
            );

            println!("{}", msg_to_arm);

            // Используем Tokio runtime для асинхронной функции в синхронном коде
            runtime.block_on(uds_write_to(stream, &msg_to_arm));
        }
        //очистка
        while let Some(_) = data_vec.pop() {}
        while let Some(_) = EARN_QUEUE.pop() {}
    }
}

fn round_to_scale(value: BigDecimal, scale: i64) -> BigDecimal {
    value.with_scale(scale)
}

fn calculate_triangle(
    spm: &HashMap<String, (String, String)>,
    key: &TriangleKey,
    t: &Vec<(String, String)>,
) -> Option<(BigDecimal, BigDecimal)> {
    // Извлекаем направления сделок из ключа
    let (pair1, dir1) = (&t[0].0, &key.d);
    let (pair2, dir2) = (&t[1].0, &t[1].1);
    let (pair3, dir3) = (&t[2].0, &t[2].1);

    let price1 = BigDecimal::from_str(&spm.get(pair1).map_or("0.0", |(price, _)| price)).unwrap();
    let price2 = BigDecimal::from_str(&spm.get(pair2).map_or("0.0", |(price, _)| price)).unwrap();
    let price3 = BigDecimal::from_str(&spm.get(pair3).map_or("0.0", |(price, _)| price)).unwrap();

    // Начинаем с 1 единицы базовой валюты
    let mut base_amount = BigDecimal::from(1);
    let base_amount_first = base_amount.clone();

    // Первая сделка
    if dir1 == "SELL" {
        base_amount = round_to_scale(base_amount * price1, 10); // базовая валюта продается
    } else if dir1 == "BUY" {
        base_amount = round_to_scale(base_amount / price1, 10); // покупаем за базовую валюту
    }

    // Вторая сделка
    if dir2 == "SELL" {
        base_amount = round_to_scale(base_amount * price2, 10); // продажа по второй цене
    } else if dir2 == "BUY" {
        base_amount = round_to_scale(base_amount / price2, 10); // покупка по второй цене
    }

    // Третья сделка
    if dir3 == "SELL" {
        base_amount = round_to_scale(base_amount * price3, 10); // продажа по третьей цене
    } else if dir3 == "BUY" {
        base_amount = round_to_scale(base_amount / price3, 10); // покупка по третьей цене
    }

    // Вычисление доходности в процентах
    let earn = round_to_scale(
        ((base_amount.clone() - base_amount_first.clone()) / base_amount.clone())
            * BigDecimal::from(100),
        2,
    );

    Some((base_amount, earn))
}

pub fn get_nodes_by_label<'a>(
    graph: &DiGraph<(&'a str, &'a str), ()>,
    label: &'a str,
) -> Vec<&'a str> {
    graph
        .node_indices()
        .filter_map(|node_index| {
            let (value, node_label) = graph[node_index];
            if node_label == label {
                Some(value)
            } else {
                None
            }
        })
        .collect()
}

pub fn depth_first_search<'a>(
    graph: &DiGraph<(&'a str, &'a str), ()>,
    base_nodes: &[&'a str],
    depth: usize,
) -> Vec<Vec<&'a str>> {
    let mut all_cycles = Vec::new();

    for &start_node in base_nodes {
        let start_index = graph
            .node_indices()
            .find(|&i| graph[i].0 == start_node)
            .expect("Start node not found");

        let mut stack = VecDeque::new();
        stack.push_back((start_index, vec![start_node]));

        while let Some((current_index, path)) = stack.pop_back() {
            if path.len() == depth {
                if graph
                    .neighbors(current_index)
                    .any(|neighbor_index| graph[neighbor_index].0 == start_node)
                {
                    let mut cycle_path = path.clone();
                    cycle_path.push(start_node);
                    all_cycles.push(cycle_path);
                }
                continue;
            }

            for neighbor_index in graph.neighbors(current_index) {
                let neighbor_value = graph[neighbor_index].0;
                if path.contains(&neighbor_value) {
                    continue;
                }

                let mut new_path = path.clone();
                new_path.push(neighbor_value);
                stack.push_back((neighbor_index, new_path));
            }
        }
    }

    all_cycles
}

/* создает уникальные и отсортированные треугольники */
pub fn triangle_sorting(all_cycles: Vec<Vec<&str>>) -> Vec<Vec<&str>> {
    let mut unique_sorted_paths = Vec::new();

    for path in all_cycles {
        // Создаем HashSet для уникальных значений
        let unique_values: HashSet<&str> = path.iter().cloned().collect();

        // Преобразуем HashSet обратно в вектор и сортируем его
        let mut unique_sorted: Vec<&str> = unique_values.into_iter().collect();
        unique_sorted.sort_unstable();

        unique_sorted_paths.push(unique_sorted);
    }

    unique_sorted_paths
}

pub fn remove_duplicates(cycles: Vec<Vec<&str>>) -> Vec<Vec<&str>> {
    let mut unique_cycles = HashSet::new();
    let mut result = Vec::new();

    for cycle in cycles {
        if unique_cycles.insert(cycle.clone()) {
            result.push(cycle);
        }
    }

    result
}

fn generate_random_id(length: usize) -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
        .chars()
        .collect();
    let mut rng = rand::thread_rng();

    iter::repeat_with(|| chars[rng.gen_range(0..chars.len())])
        .take(length)
        .collect()
}

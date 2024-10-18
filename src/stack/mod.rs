use dashmap::DashMap;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::brain::triangle::{TriangleElement, TriangleKey};

//use crate::{brain::graph::TriangleElement, triangle::TriangleKey};

pub type SymbolData = Arc<
    RwLock<
        Option<(
            String, //price
            String, //volume
        )>,
    >,
>;

pub type SymbolDataMap = Arc<DashMap<String, SymbolData>>; //symbol - SymbolData (price-volume-refs)

pub fn create_symbol_data_map(
    pairs: &Vec<String>,
    triangles: &HashMap<TriangleKey, Vec<TriangleElement>>,
) -> SymbolDataMap {
    let symbol_data_map: SymbolDataMap = Arc::new(DashMap::new());

    for symbol in pairs.iter() {
        let symbol_data: SymbolData = Arc::new(RwLock::new(None));
        symbol_data_map.insert(symbol.clone(), symbol_data.clone());

        let mut triangle_data = Vec::new();

        for (key, value) in triangles.iter() {
            if key.a == *symbol || key.b == *symbol || key.c == *symbol {
                // Создаем кортеж и добавляем его в вектор
                triangle_data.push((Arc::new(key.clone()), Arc::new(value.clone())));
            }
        }

        // Обновляем данные символа с вектором кортежей
        let mut data = symbol_data.write().unwrap();
        *data = Some((
            symbol.clone(),
            "some_data".to_string(),
            // Some(triangle_data), // Используем вектор кортежей
        ));
    }

    symbol_data_map
}

// pub fn print_symbol_data_map(data_map: &SymbolDataMap) {
//     // Итерируемся по каждому символу в DashMap
//     println!("=====================================");
//     data_map.iter().for_each(|entry| {
//         let symbol = entry.key();
//         let symbol_data = entry.value();

//         let symbol_data_read = symbol_data.read().unwrap();

//         if let Some((price, volume)) = &*symbol_data_read {
//             println!("Symbol: {}, Price: {}, Volume: {}", symbol, price, volume);
//         } else {
//             println!("Symbol: {}, No price or volume data available.", symbol);
//         }
//     });
// }

use std::collections::{HashMap, HashSet};

// use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use regex::Regex;

use crate::brain_sets::{AltCurrency, BaseCurrency, ParsedPairs};

use super::triangle::TriangleElement;

pub fn create_graph<'a>(
    mut graph: DiGraph<(&'a str, &'a str), ()>,
    base: &'a [BaseCurrency],
    alt: &'a [AltCurrency],
    clean: &'a [ParsedPairs],
) -> DiGraph<(&'a str, &'a str), ()> {
    let mut node_map = HashMap::new();

    //сначала только базовые ноды сохраним, так как метки не переписываются это позволит монетам которые могут быть и base и alt быть только base
    for base_currency in base {
        for alt_currency in alt {
            let concatenated = format!("{}{}", alt_currency.symbol, base_currency.symbol);
            if let Some(_pair) = clean.iter().find(|pair| pair.symbol == concatenated) {
                let _ = add_unique_node(
                    &mut graph,
                    &mut node_map,
                    base_currency.symbol.as_str(),
                    Some("base"),
                );
            }
        }
    }
    //теперь еще раз, но сохраним все и еще свяжем ребрами (базовые метки не будут изменены)
    for base_currency in base {
        for alt_currency in alt {
            let concatenated = format!("{}{}", alt_currency.symbol, base_currency.symbol);
            if let Some(_pair) = clean.iter().find(|pair| pair.symbol == concatenated) {
                let b = add_unique_node(
                    &mut graph,
                    &mut node_map,
                    base_currency.symbol.as_str(),
                    None,
                );
                let a = add_unique_node(
                    &mut graph,
                    &mut node_map,
                    alt_currency.symbol.as_str(),
                    Some("alt"),
                );
                graph.add_edge(b, a, ());
                graph.add_edge(a, b, ());
            }
        }
    }

    graph
}

fn add_unique_node<'a>(
    graph: &mut DiGraph<(&'a str, &'a str), ()>,
    node_map: &mut HashMap<&'a str, NodeIndex>,
    value: &'a str,
    label: Option<&'a str>,
) -> NodeIndex {
    if let Some(&node) = node_map.get(value) {
        node
    } else {
        let label_str = label.unwrap_or("");
        let node = graph.add_node((value, label_str));
        node_map.insert(value, node);
        node
    }
}

pub fn re_cycles<'a>(cycles: &Vec<Vec<&str>>, clean: &'a [ParsedPairs]) -> Vec<Vec<String>> {
    let mut result = Vec::new();

    for cycle in cycles {
        let len = cycle.len();
        let mut inner_result = Vec::new();

        for i in 0..len {
            let first = cycle[i];
            let second = cycle[(i + 1) % len];
            let concatenated = format!("{}{}", first, second);
            let reversed = format!("{}{}", second, first);

            if clean.iter().any(|pair| pair.symbol == concatenated) {
                inner_result.push(concatenated);
            } else if clean.iter().any(|pair| pair.symbol == reversed) {
                inner_result.push(reversed);
            }
        }

        result.push(inner_result);
    }

    result
}

pub fn find_differences(
    clean_pairs: &[ParsedPairs],
    need_cycles: &Vec<Vec<String>>,
) -> Vec<Vec<String>> {
    let need_set: HashSet<&str> = need_cycles
        .iter()
        .flat_map(|cycle| cycle.iter())
        .map(|s| s.as_str())
        .collect();
    let mut remaining_pairs: Vec<ParsedPairs> = clean_pairs.to_vec();
    remaining_pairs.retain(|pair| !need_set.contains(pair.symbol.as_str()));
    let differences: Vec<Vec<String>> = remaining_pairs
        .into_iter()
        .map(|pair| vec![pair.symbol])
        .collect();
    differences
}

pub fn clearing(clean_pairs: &[ParsedPairs], diff: &Vec<Vec<String>>) -> Vec<String> {
    let diff_set: HashSet<&str> = diff
        .iter()
        .flat_map(|cycle| cycle.iter())
        .map(|s| s.as_str())
        .collect();
    let mut remaining_pairs: Vec<ParsedPairs> = clean_pairs.to_vec();
    remaining_pairs.retain(|pair| !diff_set.contains(pair.symbol.as_str()));
    let clearing: Vec<String> = remaining_pairs
        .into_iter()
        .map(|pair| pair.symbol)
        .collect();
    clearing
}
/*
https://viz-js.com
*/

pub fn create_triangles(
    cycles: &Vec<String>,
    base: &Vec<BaseCurrency>,
    mut current: String,
    mut direction: String,
    mut accumulator: Vec<TriangleElement>,
) -> (Vec<TriangleElement>, String) {
    if cycles.is_empty() {
        return (accumulator, current);
    }
    if current.is_empty() {
        if direction == "SELL" {
            for base_curr in base {
                let re = Regex::new(&format!(r"^{}", base_curr.symbol)).unwrap(); //начало строки
                for element in cycles.iter() {
                    if re.is_match(element) {
                        current = base_curr.symbol.clone();
                        break;
                    }
                }
                if !current.is_empty() {
                    break;
                }
            }
        } else if direction == "BUY" {
            for base_curr in base {
                let re = Regex::new(&format!(r"{}$", base_curr.symbol)).unwrap(); //конец строки
                for element in cycles.iter() {
                    if re.is_match(element) {
                        current = base_curr.symbol.clone();
                        break;
                    }
                }
                if !current.is_empty() {
                    break;
                }
            }
        }
    }
    //про current - выше он установлен в одну из базовых валют потому что там деньги и начинаем с них
    //ниже нужно будет обновить

    // Найти первый элемент, соответствующий текущему значению по регулярному выражению
    let mut first_element = None;
    let mut first_index = None;

    for (index, element) in cycles.iter().enumerate() {
        let re_start = Regex::new(&format!(r"^{}", current)).unwrap(); // начало строки
        let re_end = Regex::new(&format!(r"{}$", current)).unwrap(); // конец строки

        if direction == "BUY" && re_end.is_match(element) {
            first_element = Some(element.clone());
            first_index = Some(index);
            break;
        } else if direction == "SELL" && re_start.is_match(element) {
            first_element = Some(element.clone());
            first_index = Some(index);
            break;
        } else {
        }
    }

    if first_element.is_none() {
        return (accumulator, current);
    }
    let first_element = first_element.unwrap();

    // новый вектор без найденного элемента
    let mut new_cycles = cycles.clone();
    if let Some(index) = first_index {
        new_cycles.remove(index);
    }

    /*
     выше - отобрали первую пару (first_element) согласно направлению и текущей валюты (current)
     правило - один символ встречается только в двух парах.
     Значит если один раз его нашли и исключили то он остался только в одной уже паре
    */
    let mut re_end = Regex::new(&format!(r"^{}", current)).unwrap(); //начало строки
    let mut re_start = Regex::new(&format!(r"{}$", current)).unwrap(); //конец строки

    /*
     нужно сменить текущую валюту (current) на вторую половину пары где был прошлый current
    */
    if re_start.is_match(&first_element) {
        current = first_element[..first_element.len() - current.len()].to_string();
    } else if re_end.is_match(&first_element) {
        current = first_element[current.len()..].to_string();
    }

    re_end = Regex::new(&format!(r"^{}", current)).unwrap(); //начало строки
    re_start = Regex::new(&format!(r"{}$", current)).unwrap(); //конец строки

    // Создаем новый элемент TriangleElement
    let modified_element = (first_element, direction.clone());
    // Добавляем его в аккумулятор
    accumulator.push(modified_element);
    /*
    сменить направление если нужно
     */
    if new_cycles.len() == 0 {
        return (accumulator, current);
    }

    for nc in &new_cycles {
        if re_end.is_match(&nc) {
            direction = "SELL".to_string();
            break;
        } else if re_start.is_match(&nc) {
            direction = "BUY".to_string();
            break;
        } else {
        }
    }
    // Рекурсивный вызов с обновленным current/direction
    create_triangles(&new_cycles, base, current, direction.clone(), accumulator)
}

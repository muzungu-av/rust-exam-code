use std::fmt;

// Определение структуры для ключа
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct TriangleKey {
    pub a: String,
    pub b: String,
    pub c: String,
    pub d: String,
} //Triangle Key: TriangleKey { a: "ETHBTC", b: "ETHUSDT", c: "BTCUSDT", d: "SELL" }

impl fmt::Display for TriangleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Определяем, как выводить поля структуры
        write!(
            f,
            "TriangleKey(a: {}, b: {}, c: {}, d: {})",
            self.a, self.b, self.c, self.d
        )
    }
}

pub type TriangleElement = (
    String, // symbol
    String, // direction
);

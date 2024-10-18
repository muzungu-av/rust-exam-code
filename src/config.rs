use dotenv::from_filename;
use std::env;

pub struct Config {
    pub tracing_on: bool,
    pub ping_interval: u16,
    pub brain: String,
    pub reader_count: u8,
    pub wss_url: String,
    pub http_port: u16,
    pub volume_accept: bool,
    pub auto_subscription: bool,
    pub response_rate: f64,
}

pub async fn init() -> Config {
    let env_file =
        env::var("PRODUCTION_ENV_FILE").unwrap_or_else(|_| ".env.development".to_string());
    from_filename(&env_file).ok();

    let tracing_on_str = env::var("tracing_on").unwrap_or("false".to_string());
    let tracing_on: bool = tracing_on_str.parse().unwrap_or(false);

    let ping_interval: u16 = env::var("ping_interval")
        .expect("PING_INTERVAL must be set")
        .parse()
        .expect("PING_INTERVAL must be a valid u16 number");

    let reader_count: u8 = env::var("reader_count")
        .expect("READER_COUNT must be set")
        .parse()
        .expect("READER_COUNT must be a valid u8 number");

    let port_str = env::var("http_port").unwrap_or("8080".to_string());
    let http_port: u16 = port_str.parse().unwrap_or(8080);

    let wss_url = env::var("wss_url").expect("URL must be set");

    let brain = env::var("brain").expect("BRAIN must be set");

    let volume_accept_str = env::var("volume_accept").unwrap_or("false".to_string());
    let volume_accept: bool = volume_accept_str.parse().unwrap_or(false);

    let auto_subscription_str = env::var("auto_subscription").unwrap_or("false".to_string());
    let auto_subscription: bool = auto_subscription_str.parse().unwrap_or(false);

    let response_rate_str = env::var("response_rate").unwrap_or("1.0".to_string());
    let response_rate: f64 = response_rate_str.parse().unwrap_or(1.0);

    Config {
        tracing_on,
        ping_interval,
        reader_count,
        http_port,
        wss_url,
        brain,
        volume_accept,
        auto_subscription,
        response_rate,
    }
}

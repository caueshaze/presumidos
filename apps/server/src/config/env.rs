use std::path::Path;

pub(crate) fn required_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("variavel {name} ausente no .env"))
}

#[cfg(feature = "server")]
pub(crate) fn parse_bool_var(name: &str) -> bool {
    match required_var(name).trim().to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => panic!("variavel {name} deve ser booleana"),
    }
}

#[cfg(feature = "server")]
pub(crate) fn parse_i64_var(name: &str) -> i64 {
    required_var(name)
        .trim()
        .parse::<i64>()
        .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica"))
}

#[cfg(feature = "server")]
pub(crate) fn parse_cidr_list_var(name: &str) -> Vec<ipnet::IpNet> {
    required_var(name)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<ipnet::IpNet>()
                .unwrap_or_else(|_| panic!("variavel {name} contem CIDR invalido: {value}"))
        })
        .collect()
}

#[cfg(feature = "server")]
pub(crate) fn optional_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(feature = "server")]
pub(crate) fn optional_u32_var(name: &str, default: u32) -> u32 {
    match optional_var(name) {
        Some(value) => value
            .parse::<u32>()
            .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica")),
        None => default,
    }
}

#[cfg(feature = "server")]
pub(crate) fn optional_u64_var(name: &str, default: u64) -> u64 {
    match optional_var(name) {
        Some(value) => value
            .parse::<u64>()
            .unwrap_or_else(|_| panic!("variavel {name} deve ser numerica")),
        None => default,
    }
}

#[cfg(feature = "server")]
pub(crate) fn optional_bool_var(name: &str, default: bool) -> bool {
    match optional_var(name) {
        Some(value) => match value.trim().to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => panic!("variavel {name} deve ser booleana"),
        },
        None => default,
    }
}

#[cfg(feature = "server")]
pub(crate) fn default_asset_dir(database_path: &str) -> String {
    let database_path = Path::new(database_path);
    if database_path.is_absolute() {
        database_path
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .join("data/assets")
            .to_string_lossy()
            .into_owned()
    } else {
        "./data/assets".to_string()
    }
}

use std::{env, fs, path::PathBuf};

fn main() {
    const CONFIG: &str = "aeris.config.toml";
    const COLOR_KEYS: [&str; 4] = ["background", "primary_text", "secondary_text", "accent"];

    println!("cargo:rerun-if-changed={CONFIG}");
    let source = fs::read_to_string(CONFIG).expect("could not read aeris.config.toml");
    let config: toml::value::Table =
        toml::from_str(&source).expect("invalid TOML in aeris.config.toml");
    let theme = config
        .get("theme")
        .and_then(toml::Value::as_table)
        .expect("aeris.config.toml requires a [theme] table");
    for key in theme.keys() {
        assert!(key == "button_style" || key == "colors", "{CONFIG}: unknown key {key}");
    }
    let style = match theme.get("button_style").and_then(toml::Value::as_str) {
        Some("glass") => "Glass",
        Some("flat") => "Normal",
        _ => panic!("{CONFIG}: button_style must be glass or flat"),
    };
    let mut colors = COLOR_KEYS.map(|_| String::from("None"));
    if let Some(configured) = theme.get("colors") {
        let configured = configured.as_table().expect("colors must be a table");
        for (key, value) in configured {
            let index = COLOR_KEYS
                .iter()
                .position(|&name| name == key.as_str())
                .unwrap_or_else(|| panic!("{CONFIG}: unknown color {key}"));
            let hex = value.as_str().unwrap_or_default();
            assert!(
                hex.len() == 7
                    && hex.starts_with('#')
                    && hex.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit),
                "{CONFIG}: colors.{key} must use #RRGGBB"
            );
            colors[index] = format!("theme::Rgb::parse(b{hex:?})");
        }
    }
    let colors = colors.join(", ");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        output.join("aeris_theme.rs"),
        format!(
            "fn apply_theme() {{
                use aeris_ui::theme::{{self, ButtonTheme}};
                theme::apply(ButtonTheme::{style}, [{colors}]);
            }}\n"
        ),
    )
    .unwrap();
}

use std::{env, fmt::Write, fs, path::PathBuf};

fn main() {
    let source = "display-profiles.json";
    println!("cargo:rerun-if-changed={source}");
    let profiles: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(source).unwrap()).unwrap();
    let profiles = profiles.as_array().unwrap();
    let base_apx = profiles[0]["apx4"].as_u64().unwrap();
    let mut generated = format!(
        "const BASE_APX_SCALE: ApxScale = match ApxScale::new({base_apx}) {{ \
         Some(scale) => scale, None => panic!(\"invalid apx scale\") }};\n\
         pub const DISPLAY_PROFILES: &[DisplayProfile] = &[\n"
    );
    for profile in profiles {
        let id = profile["id"].as_str().unwrap();
        let name = profile["name"].as_str().unwrap();
        let width = profile["width"].as_i64().unwrap();
        let height = profile["height"].as_i64().unwrap();
        let apx4 = profile["apx4"].as_u64().unwrap();
        let shape = match profile["shape"].as_str().unwrap() {
            "rectangle" => "Rectangle",
            "capsule" => "Capsule",
            "circle" => "Circle",
            shape => panic!("unknown display shape: {shape}"),
        };
        let safe = profile["safe"].as_array().unwrap();
        let [x, y, w, h] = [0, 1, 2, 3].map(|index| safe[index].as_i64().unwrap());
        writeln!(
            generated,
            "DisplayProfile {{ id: {id:?}, name: {name:?}, shape: ScreenShape::{shape}, \
             viewport: match Viewport::new(Rect::new(0, 0, {width}, {height}), Rect::new({x}, {y}, {w}, {h})) {{ \
             Some(value) => value.with_apx_scale(match ApxScale::new({apx4}) {{ \
             Some(scale) => scale, None => panic!(\"invalid apx scale\") }}), \
             None => panic!(\"invalid display geometry\") }} }},"
        )
        .unwrap();
    }
    generated.push_str("];\n");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("display_profiles.rs");
    fs::write(output, generated).unwrap();
}

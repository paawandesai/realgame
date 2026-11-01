//! The material table (section 5.2): `content/materials.json`, loaded and
//! checked here. One row per material id (`crate::mat`), in id order.
//!
//! Physical values may be the string `"todo"` until Lane B fills and sources
//! them (V10 then forbids any `"todo"`); a field that does not apply is
//! `null`. Gases (CO2, H2, CH4, O2) are not rows: they live in the matter
//! crate's gas field.

use serde_json::Value;

/// The table, compiled in (the client never downloads it).
pub const MATERIALS_JSON: &str = include_str!("../../../content/materials.json");

/// Physical state at ambient conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Solid,
    Liquid,
    Gas,
    /// Air: no material.
    None,
}

/// A numeric field: a value, not applicable (`null`), or still `"todo"`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Num {
    Set(f32),
    Null,
    Todo,
}

impl Num {
    /// The value, if set.
    pub fn get(self) -> Option<f32> {
        match self {
            Num::Set(v) => Some(v),
            _ => None,
        }
    }
}

/// One row of the table.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub id: u8,
    pub name: String,
    pub phase: Phase,
    pub density_kg_m3: Num,
    pub melt_c: Num,
    pub boil_c: Num,
    pub heat_capacity_j_kgk: Num,
    pub conductivity_w_mk: Num,
    /// 0-1.
    pub flammability: Num,
    pub ignite_c: Num,
    pub burn_s: Num,
    /// Dig time in seconds.
    pub hardness_s: Num,
    /// The item name a dig adds to the pack (`None`: nothing).
    pub drops: Option<String>,
    /// Tile index in the 16 x 16 atlas.
    pub atlas: u8,
    pub transparent: bool,
    pub solid: bool,
    /// Key into `content/science-sources.md` (`None` while "todo").
    pub source: Option<String>,
}

/// The numeric fields, in schema order (for the "todo" scan and V10).
pub const NUM_FIELDS: [&str; 9] = [
    "density_kg_m3",
    "melt_c",
    "boil_c",
    "heat_capacity_j_kgk",
    "conductivity_w_mk",
    "flammability",
    "ignite_c",
    "burn_s",
    "hardness_s",
];

fn num(row: &Value, key: &str) -> Result<Num, String> {
    match row.get(key) {
        Some(Value::Number(n)) => Ok(Num::Set(n.as_f64().unwrap_or(f64::NAN) as f32)),
        Some(Value::Null) => Ok(Num::Null),
        Some(Value::String(s)) if s == "todo" => Ok(Num::Todo),
        Some(v) => Err(format!(
            "{key}: expected a number, null or \"todo\", got {v}"
        )),
        None => Err(format!("{key}: missing")),
    }
}

fn boolean(row: &Value, key: &str) -> Result<bool, String> {
    row.get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{key}: expected true or false"))
}

fn text(row: &Value, key: &str) -> Result<Option<String>, String> {
    match row.get(key) {
        Some(Value::String(s)) if s == "todo" => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(Value::Null) => Ok(None),
        Some(v) => Err(format!("{key}: expected a string or null, got {v}")),
        None => Err(format!("{key}: missing")),
    }
}

fn row(v: &Value) -> Result<Material, String> {
    let name = v
        .get("name")
        .and_then(Value::as_str)
        .ok_or("name: missing")?
        .to_string();
    let at = |e: String| format!("{name}: {e}");
    let id = v
        .get("id")
        .and_then(Value::as_u64)
        .filter(|&i| i <= u8::MAX as u64)
        .ok_or_else(|| at("id: expected 0-255".into()))? as u8;
    let phase = match v.get("phase").and_then(Value::as_str) {
        Some("solid") => Phase::Solid,
        Some("liquid") => Phase::Liquid,
        Some("gas") => Phase::Gas,
        Some("none") => Phase::None,
        _ => return Err(at("phase: expected solid, liquid, gas or none".into())),
    };
    let atlas = v
        .get("atlas")
        .and_then(Value::as_u64)
        .filter(|&i| i < 256)
        .ok_or_else(|| at("atlas: expected 0-255".into()))? as u8;
    Ok(Material {
        id,
        phase,
        density_kg_m3: num(v, "density_kg_m3").map_err(at)?,
        melt_c: num(v, "melt_c").map_err(at)?,
        boil_c: num(v, "boil_c").map_err(at)?,
        heat_capacity_j_kgk: num(v, "heat_capacity_j_kgk").map_err(at)?,
        conductivity_w_mk: num(v, "conductivity_w_mk").map_err(at)?,
        flammability: num(v, "flammability").map_err(at)?,
        ignite_c: num(v, "ignite_c").map_err(at)?,
        burn_s: num(v, "burn_s").map_err(at)?,
        hardness_s: num(v, "hardness_s").map_err(at)?,
        drops: text(v, "drops").map_err(at)?,
        atlas,
        transparent: boolean(v, "transparent").map_err(at)?,
        solid: boolean(v, "solid").map_err(at)?,
        source: text(v, "source").map_err(at)?,
        name,
    })
}

/// Parses the table (`{"schema": {...}, "rows": [...]}`). Fails on a
/// malformed row or a row whose `id` is not its position.
pub fn load(json: &str) -> Result<Vec<Material>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let rows = v
        .get("rows")
        .and_then(Value::as_array)
        .ok_or("rows: missing")?;
    let out = rows.iter().map(row).collect::<Result<Vec<_>, _>>()?;
    for (i, m) in out.iter().enumerate() {
        if m.id as usize != i {
            return Err(format!("{}: id {} at position {i}", m.name, m.id));
        }
    }
    Ok(out)
}

/// The fields of each row still marked `"todo"` (for V10 and Lane B).
pub fn todos(json: &str) -> Result<Vec<(String, String)>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in v
        .get("rows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = r.get("name").and_then(Value::as_str).unwrap_or("?");
        if let Some(obj) = r.as_object() {
            for (k, val) in obj {
                if val.as_str() == Some("todo") {
                    out.push((name.to_string(), k.clone()));
                }
            }
        }
    }
    Ok(out)
}

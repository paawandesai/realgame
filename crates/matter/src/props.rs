//! Per-material numbers the matter step uses, read once from
//! `content/materials.json` (through `realgame_voxel::materials`) and turned
//! into plain `f32`s, so a step never parses or looks anything up by name.

use realgame_voxel::mat;
use realgame_voxel::materials;

/// 0 °C in kelvin.
pub const MELT_K: f32 = 273.15;
/// 100 °C in kelvin.
pub const BOIL_K: f32 = 373.15;
/// Ambient air (and every material's resting temperature unless set below).
pub const AMBIENT_K: f32 = 293.15;
/// Resting temperature of ice and snow: the Frozen Fields' air, −5 °C
/// (scenario assumption; section 7 holds the ford's air at −5 °C).
pub const COLD_REST_K: f32 = 268.15;
/// Resting temperature of liquid water: a cold stream, 12 °C (scenario
/// assumption; above the 10 °C Leidenfrost line of section 5.3).
pub const WATER_REST_K: f32 = 285.15;
/// Specific latent heat of fusion of water, J/kg (6.01 kJ/mol ÷ 18.02 g/mol).
pub const LF_J_KG: f32 = 3.34e5;
/// Specific latent heat of vaporisation of water, J/kg (40.7 kJ/mol ÷ 18.02).
pub const LV_J_KG: f32 = 2.26e6;

/// One material's numbers.
#[derive(Clone, Copy, Debug)]
pub struct Props {
    /// Volumetric heat capacity of a full block, J/K (density × c_p × 1 m³).
    pub c_vol: f32,
    /// Thermal conductivity, W/(m·K).
    pub k: f32,
    pub density: f32,
    /// Ignition temperature in kelvin, if it burns.
    pub ignite_k: Option<f32>,
    /// Steps a burning cell burns.
    pub burn_ticks: u16,
    /// Blocks the player and rays.
    pub solid: bool,
    /// The temperature an untouched cell (`temp` 0) has, and the one a hot
    /// or cold cell relaxes back to.
    pub rest_k: f32,
}

/// The table, indexed by material id. Rows the JSON leaves `"todo"` or
/// `null` get neutral stand-ins (stone-like), so a half-filled table never
/// panics a step.
pub fn table() -> Vec<Props> {
    let rows = materials::load(materials::MATERIALS_JSON).unwrap_or_default();
    (0..256)
        .map(|id| {
            let r = rows.get(id);
            let num = |f: fn(&materials::Material) -> materials::Num, d: f32| {
                r.and_then(|r| f(r).get()).unwrap_or(d)
            };
            let density = num(|r| r.density_kg_m3, 2000.0);
            let cp = num(|r| r.heat_capacity_j_kgk, 800.0);
            let flam = num(|r| r.flammability, 0.0);
            let ignite_c = r.and_then(|r| r.ignite_c.get());
            let burn_s = num(|r| r.burn_s, 0.0);
            let id = id as u8;
            let rest_k = match id {
                mat::ICE | mat::SNOW => COLD_REST_K,
                mat::WATER => WATER_REST_K,
                _ => AMBIENT_K,
            };
            Props {
                c_vol: (density * cp).max(1.0),
                k: num(|r| r.conductivity_w_mk, 1.0).max(0.001),
                density,
                ignite_k: (flam > 0.0).then(|| ignite_c.unwrap_or(300.0) + 273.15),
                burn_ticks: (burn_s * 10.0).round().clamp(1.0, u16::MAX as f32) as u16,
                solid: r.map_or(id != mat::AIR, |r| r.solid),
                rest_k,
            }
        })
        .collect()
}

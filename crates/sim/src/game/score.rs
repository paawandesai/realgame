//! Trial scoring and stars (DESIGN_V3 §5). Track B owns this file after
//! Hour 0. Deterministic: no clocks, no randomness.

/// The solution signature: a 32-bit FNV-1a hash of the sorted, de-duplicated
/// reaction ids that changed goal-relevant cells, plus the goal mechanism id.
/// Two solutions are "different" (star 3) when their signatures differ.
pub fn solution_sig(reaction_ids: &[&str], mechanism: &str) -> u32 {
    let mut ids: Vec<&str> = reaction_ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let mut h: u32 = 0x811c_9dc5;
    let mut eat = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    };
    for id in ids {
        eat(id.as_bytes());
        eat(b"\x1f");
    }
    eat(b"\x1e");
    eat(mechanism.as_bytes());
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_ignores_order_and_repeats_but_not_mechanism() {
        let a = solution_sig(&["h2o2_decomp", "mg_co2", "h2o2_decomp"], "bridge");
        let b = solution_sig(&["mg_co2", "h2o2_decomp"], "bridge");
        assert_eq!(a, b);
        assert_ne!(a, solution_sig(&["mg_co2", "h2o2_decomp"], "platform"));
        assert_ne!(a, solution_sig(&["h2o2_decomp"], "bridge"));
    }
}

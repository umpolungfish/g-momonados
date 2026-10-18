//! modular_residue.rs — modular exponentiation with period certification.

fn mod_pow_u64(mut a: u64, mut e: u64, n: u64) -> u64 {
    if n == 1 { return 0; }
    let mut r = 1u64;
    a %= n;
    while e > 0 {
        if e & 1 == 1 {
            r = (r as u128 * a as u128 % n as u128) as u64;
        }
        a = (a as u128 * a as u128 % n as u128) as u64;
        e >>= 1;
    }
    r
}

/// Return `base^exp mod modulus`, certified against `period`:
/// fails if `base^period mod modulus != 1`.
pub fn certified_modular_residue_u64_step(
    exp: u64,
    base: u64,
    modulus: u64,
    period: u64,
) -> Result<u64, String> {
    if modulus == 0 {
        return Err("modulus is zero".into());
    }
    if period == 0 {
        return Err("period is zero".into());
    }
    let check = mod_pow_u64(base, period, modulus);
    if check != 1 {
        return Err(format!(
            "certification failed: {} ^ {} mod {} = {} (expected 1)",
            base, period, modulus, check,
        ));
    }
    Ok(mod_pow_u64(base, exp, modulus))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certifies_known_period() {
        // 2^6 ≡ 1 (mod 21); 2^3 = 8 (mod 21)
        assert_eq!(certified_modular_residue_u64_step(3, 2, 21, 6).unwrap(), 8);
    }

    #[test]
    fn rejects_wrong_period() {
        // 2^5 = 32 ≡ 11 (mod 21), not 1
        assert!(certified_modular_residue_u64_step(2, 2, 21, 5).is_err());
    }

    #[test]
    fn edge_case_n_equals_1() {
        // modulus 1 → result always 0
        assert_eq!(certified_modular_residue_u64_step(5, 3, 1, 1).unwrap(), 0);
    }
}
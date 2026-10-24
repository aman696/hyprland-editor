use hyprland_editor_core::lua::replay::Observed;
#[test]
fn finite_observations_preserve_every_binary64_bit_through_worker_json() {
    let mut bits = 0x1234_5678_90ab_cdef_u64;
    for _ in 0..10_000 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
        let n = f64::from_bits(bits);
        if !n.is_finite() {
            continue;
        }
        let encoded = serde_json::to_vec(&Observed::Float(n)).unwrap();
        let Observed::Float(decoded) = serde_json::from_slice(&encoded).unwrap() else {
            panic!("Wrong observation type")
        };
        assert_eq!(decoded.to_bits(), bits, "{n:?}");
    }
}

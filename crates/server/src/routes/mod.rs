pub mod admin;
pub mod paginas;
pub mod palpites;
pub mod ranking;
pub mod sse;

/// Pontuação de um palpite frente ao resultado real.
/// - Acerto exato (placar idêntico): 10 pontos
/// - Acerto do vencedor OU do empate: 5 pontos
/// - Errou: 0 pontos
pub fn calcular_pontos(p_a: i16, p_b: i16, r_a: i16, r_b: i16) -> i16 {
    if p_a == r_a && p_b == r_b {
        return 10;
    }
    if vencedor(p_a, p_b) == vencedor(r_a, r_b) {
        return 5;
    }
    0
}

/// 1 = time A vence, -1 = time B vence, 0 = empate.
fn vencedor(a: i16, b: i16) -> i8 {
    match a.cmp(&b) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acerto_exato_vale_10() {
        assert_eq!(calcular_pontos(2, 1, 2, 1), 10);
    }

    #[test]
    fn acerto_do_vencedor_vale_5() {
        assert_eq!(calcular_pontos(3, 0, 2, 1), 5);
    }

    #[test]
    fn acerto_do_empate_vale_5() {
        assert_eq!(calcular_pontos(1, 1, 2, 2), 5);
    }

    #[test]
    fn erro_vale_0() {
        assert_eq!(calcular_pontos(0, 2, 2, 1), 0);
    }
}

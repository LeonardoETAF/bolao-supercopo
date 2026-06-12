pub mod admin;
pub mod paginas;
pub mod palpites;
pub mod ranking;
pub mod sse;

/// Pontuação de um palpite frente ao resultado real.
/// - Acerto exato (placar idêntico): 10 pontos
/// - Acerto apenas do vencedor: 5 pontos
/// - Errou: 0 pontos
///
/// Empate só pontua no acerto exato do placar: um palpite de empate que não
/// crava o placar é considerado erro (não há "vencedor" a acertar). Da mesma
/// forma, palpitar empate num jogo que teve vencedor (ou vice-versa) é erro.
pub fn calcular_pontos(p_a: i16, p_b: i16, r_a: i16, r_b: i16) -> i16 {
    if p_a == r_a && p_b == r_b {
        return 10;
    }
    let vencedor_real = vencedor(r_a, r_b);
    // Só vale 5 quando houve um vencedor de fato e o palpite acertou esse vencedor.
    if vencedor_real != 0 && vencedor(p_a, p_b) == vencedor_real {
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
    fn empate_exato_vale_10() {
        assert_eq!(calcular_pontos(2, 2, 2, 2), 10);
    }

    #[test]
    fn empate_inexato_vale_0() {
        // Palpitou empate (1x1), o jogo empatou em outro placar (2x2): não crava
        // o placar e empate não tem "vencedor" a acertar -> erro.
        assert_eq!(calcular_pontos(1, 1, 2, 2), 0);
    }

    #[test]
    fn palpite_empate_em_jogo_com_vencedor_vale_0() {
        assert_eq!(calcular_pontos(1, 1, 2, 1), 0);
    }

    #[test]
    fn palpite_com_vencedor_em_jogo_que_empatou_vale_0() {
        assert_eq!(calcular_pontos(2, 1, 1, 1), 0);
    }

    #[test]
    fn erro_vale_0() {
        assert_eq!(calcular_pontos(0, 2, 2, 1), 0);
    }
}

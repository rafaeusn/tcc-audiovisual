//! Contas de calendário, sem depender de biblioteca de datas.

/// Dia local de um instante, contado em dias desde 01/01/1970.
pub fn dia_local(instante: i64, fuso_s: i64) -> i64 {
    (instante + fuso_s).div_euclid(86_400)
}

/// (ano, mês, dia) de um dia contado desde 01/01/1970 (algoritmo de Howard Hinnant, calendário gregoriano).
pub fn data_civil(dias: i64) -> (i32, u32, u32) {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mes = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let ano = (yoe + era * 400 + i64::from(mes <= 2)) as i32;
    (ano, mes, dia)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn datas_conhecidas() {
        assert_eq!(data_civil(0), (1970, 1, 1));
        assert_eq!(data_civil(-1), (1969, 12, 31));
        assert_eq!(data_civil(11_016), (2000, 2, 29), "ano bissexto");
        assert_eq!(data_civil(20_769), (2026, 11, 12));
    }

    #[test]
    fn dia_local_respeita_o_fuso() {
        // 01:00 UTC de 12/11/2026 ainda é dia 11 em Brasília.
        let instante = 20_769 * 86_400 + 3600;
        assert_eq!(dia_local(instante, 0), 20_769);
        assert_eq!(dia_local(instante, -3 * 3600), 20_768);
        assert_eq!(dia_local(-1, 0), -1);
    }
}

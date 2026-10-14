use pdfcorte::{
    ErrorCorte, Opciones, Rango, Resto, calcular_rangos, calcular_rangos_por_cortes,
    cortes_a_tamanos, nombre_parte, parsear_corte, parsear_tamano,
};

fn r(inicio: u32, fin: u32) -> Rango {
    Rango::new(inicio, fin)
}

fn opciones(resto: Resto, repetir: bool, truncar: bool) -> Opciones {
    Opciones {
        resto,
        repetir,
        truncar,
    }
}

#[test]
fn suma_exacta() {
    let rangos = calcular_rangos(10, &[2, 3, 5], &Opciones::default()).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 10)]);
}

#[test]
fn sobrante_incluir_por_defecto() {
    let rangos = calcular_rangos(12, &[2, 3, 5], &Opciones::default()).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 10), r(11, 12)]);
}

#[test]
fn sobrante_descartar() {
    let o = opciones(Resto::Descartar, false, false);
    let rangos = calcular_rangos(12, &[2, 3, 5], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 10)]);
}

#[test]
fn sobrante_error() {
    let o = opciones(Resto::Error, false, false);
    let err = calcular_rangos(12, &[2, 3, 5], &o).unwrap_err();
    assert!(matches!(err, ErrorCorte::PaginasSobrantes { inicio: 11, fin: 12 }));
    assert!(err.to_string().contains("sobran 2 páginas"), "{err}");
}

#[test]
fn mensajes_en_singular() {
    let o = opciones(Resto::Error, false, false);
    let err = calcular_rangos(3, &[2], &o).unwrap_err();
    assert_eq!(err.to_string(), "sobra 1 página (p3-3) y se indicó --resto error");

    let err = calcular_rangos(1, &[2], &Opciones::default()).unwrap_err();
    assert!(err.to_string().starts_with("el PDF tiene 1 página,"), "{err}");
}

#[test]
fn resto_error_sin_sobrante_es_ok() {
    let o = opciones(Resto::Error, false, false);
    assert_eq!(calcular_rangos(5, &[2, 3], &o).unwrap(), vec![r(1, 2), r(3, 5)]);
}

#[test]
fn exceso_sin_truncar_falla_con_totales() {
    let err = calcular_rangos(8, &[2, 3, 5], &Opciones::default()).unwrap_err();
    assert!(matches!(
        err,
        ErrorCorte::PaginasInsuficientes {
            total: 8,
            pedidas: 10
        }
    ));
    let msg = err.to_string();
    assert!(msg.contains("8 páginas") && msg.contains("suman 10"), "{msg}");
}

#[test]
fn exceso_con_truncar_recorta_ultimo_bloque() {
    let o = opciones(Resto::Incluir, false, true);
    let rangos = calcular_rangos(8, &[2, 3, 5], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 8)]);
}

#[test]
fn truncar_descarta_bloques_sin_paginas() {
    let o = opciones(Resto::Error, false, true);
    let rangos = calcular_rangos(5, &[2, 3, 5, 1], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5)]);
}

#[test]
fn truncar_sin_exceso_no_cambia_nada() {
    let o = opciones(Resto::Incluir, false, true);
    let rangos = calcular_rangos(12, &[2, 3, 5], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 10), r(11, 12)]);
}

#[test]
fn repetir_tamano_uno() {
    let o = opciones(Resto::Incluir, true, false);
    let rangos = calcular_rangos(4, &[1], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 1), r(2, 2), r(3, 3), r(4, 4)]);
}

#[test]
fn repetir_tamano_dos_exacto() {
    let o = opciones(Resto::Error, true, false);
    let rangos = calcular_rangos(6, &[2], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 4), r(5, 6)]);
}

#[test]
fn repetir_secuencia_con_resto_incluido() {
    let o = opciones(Resto::Incluir, true, false);
    let rangos = calcular_rangos(12, &[2, 3], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 2), r(3, 5), r(6, 7), r(8, 10), r(11, 12)]);
}

#[test]
fn repetir_bloque_incompleto_es_resto() {
    let o = opciones(Resto::Incluir, true, false);
    assert_eq!(
        calcular_rangos(11, &[2, 3], &o).unwrap(),
        vec![r(1, 2), r(3, 5), r(6, 7), r(8, 10), r(11, 11)]
    );

    let o = opciones(Resto::Descartar, true, false);
    assert_eq!(
        calcular_rangos(11, &[2, 3], &o).unwrap(),
        vec![r(1, 2), r(3, 5), r(6, 7), r(8, 10)]
    );

    let o = opciones(Resto::Error, true, false);
    assert!(matches!(
        calcular_rangos(11, &[2, 3], &o),
        Err(ErrorCorte::PaginasSobrantes { inicio: 11, fin: 11 })
    ));
}

#[test]
fn repetir_bloque_mayor_que_documento() {
    let o = opciones(Resto::Incluir, true, false);
    assert_eq!(calcular_rangos(3, &[5], &o).unwrap(), vec![r(1, 3)]);

    let o = opciones(Resto::Descartar, true, false);
    assert!(matches!(calcular_rangos(3, &[5], &o), Err(ErrorCorte::SinPartes)));
}

#[test]
fn tamano_cero_rechazado() {
    let err = calcular_rangos(10, &[2, 0, 3], &Opciones::default()).unwrap_err();
    assert!(matches!(err, ErrorCorte::TamanoInvalido(_)));
}

#[test]
fn lista_vacia_rechazada() {
    let err = calcular_rangos(10, &[], &Opciones::default()).unwrap_err();
    assert!(matches!(err, ErrorCorte::TamanosVacios));
}

#[test]
fn documento_sin_paginas_rechazado() {
    let err = calcular_rangos(0, &[1], &Opciones::default()).unwrap_err();
    assert!(matches!(err, ErrorCorte::DocumentoVacio));
}

#[test]
fn suma_enorme_no_desborda() {
    let err = calcular_rangos(10, &[u32::MAX, u32::MAX], &Opciones::default()).unwrap_err();
    assert!(matches!(err, ErrorCorte::PaginasInsuficientes { .. }));

    let o = opciones(Resto::Incluir, false, true);
    assert_eq!(calcular_rangos(10, &[u32::MAX, 1], &o).unwrap(), vec![r(1, 10)]);
}

#[test]
fn parsear_tamanos_validos_e_invalidos() {
    assert_eq!(parsear_tamano("3").unwrap(), 3);
    assert_eq!(parsear_tamano(" 7 ").unwrap(), 7);
    for invalido in ["", " ", "0", "-1", "abc", "2.5", "99999999999"] {
        assert!(
            matches!(parsear_tamano(invalido), Err(ErrorCorte::TamanoInvalido(_))),
            "debería rechazar «{invalido}»"
        );
    }
}

#[test]
fn cortes_a_tamanos_diferencias() {
    assert_eq!(cortes_a_tamanos(&[22, 38, 53]).unwrap(), vec![22, 16, 15]);
    assert_eq!(cortes_a_tamanos(&[1]).unwrap(), vec![1]);
}

#[test]
fn cortes_a_tamanos_rechaza_invalidos() {
    assert!(matches!(cortes_a_tamanos(&[]), Err(ErrorCorte::CortesVacios)));
    assert!(matches!(cortes_a_tamanos(&[0, 5]), Err(ErrorCorte::CorteInvalido(_))));

    let err = cortes_a_tamanos(&[22, 69, 53]).unwrap_err();
    assert!(matches!(err, ErrorCorte::CorteNoCreciente { anterior: 69, corte: 53 }));
    assert_eq!(err.to_string(), "el corte 53 debe ser mayor que 69");

    let err = cortes_a_tamanos(&[10, 10]).unwrap_err();
    assert_eq!(err.to_string(), "el corte 10 debe ser mayor que 10");
}

#[test]
fn parsear_cortes() {
    assert_eq!(parsear_corte(" 38").unwrap(), 38);
    for invalido in ["", "0", "-4", "x"] {
        assert!(
            matches!(parsear_corte(invalido), Err(ErrorCorte::CorteInvalido(_))),
            "debería rechazar «{invalido}»"
        );
    }
}

#[test]
fn rangos_por_cortes() {
    let rangos = calcular_rangos_por_cortes(53, &[22, 38, 53], &Opciones::default()).unwrap();
    assert_eq!(rangos, vec![r(1, 22), r(23, 38), r(39, 53)]);
}

#[test]
fn rangos_por_cortes_con_resto() {
    let rangos = calcular_rangos_por_cortes(60, &[22, 38, 53], &Opciones::default()).unwrap();
    assert_eq!(rangos, vec![r(1, 22), r(23, 38), r(39, 53), r(54, 60)]);

    let o = opciones(Resto::Descartar, false, false);
    let rangos = calcular_rangos_por_cortes(60, &[22, 38, 53], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 22), r(23, 38), r(39, 53)]);

    let o = opciones(Resto::Error, false, false);
    let err = calcular_rangos_por_cortes(60, &[22, 38, 53], &o).unwrap_err();
    assert!(matches!(err, ErrorCorte::PaginasSobrantes { inicio: 54, fin: 60 }));
}

#[test]
fn corte_mayor_que_total_sin_truncar() {
    let err = calcular_rangos_por_cortes(50, &[22, 38, 53], &Opciones::default()).unwrap_err();
    assert!(matches!(err, ErrorCorte::CorteExcedeTotal { corte: 53, total: 50 }));
    let msg = err.to_string();
    assert!(msg.contains("corte 53") && msg.contains("50 páginas"), "{msg}");
}

#[test]
fn corte_mayor_que_total_con_truncar() {
    let o = opciones(Resto::Error, false, true);
    let rangos = calcular_rangos_por_cortes(30, &[22, 38, 53], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 22), r(23, 30)]);

    // Si el corte anterior ya llega al final, el recortado no genera parte vacía.
    let rangos = calcular_rangos_por_cortes(38, &[22, 38, 53], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 22), r(23, 38)]);
}

#[test]
fn cortes_ignoran_repetir() {
    let o = opciones(Resto::Incluir, true, false);
    let rangos = calcular_rangos_por_cortes(10, &[3], &o).unwrap();
    assert_eq!(rangos, vec![r(1, 3), r(4, 10)]);
}

#[test]
fn orden_se_valida_antes_que_el_total() {
    let o = opciones(Resto::Incluir, false, true);
    let err = calcular_rangos_por_cortes(30, &[22, 69, 53], &o).unwrap_err();
    assert!(matches!(err, ErrorCorte::CorteNoCreciente { anterior: 69, corte: 53 }));
}

#[test]
fn nombres_con_relleno() {
    assert_eq!(nombre_parte("factura", 1, 3, r(1, 2)), "factura_parte01_p1-2.pdf");
    assert_eq!(nombre_parte("f", 12, 12, r(23, 24)), "f_parte12_p23-24.pdf");
    assert_eq!(nombre_parte("f", 7, 150, r(7, 7)), "f_parte007_p7-7.pdf");
}

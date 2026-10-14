mod common;

use std::fs;
use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::TempDir;

use common::{escribir_pdf, verificar_parte};

fn pdfcorte() -> Command {
    cargo_bin_cmd!("pdfcorte")
}

fn stdout(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stdout).into_owned()
}

fn stderr(salida: &Output) -> String {
    String::from_utf8_lossy(&salida.stderr).into_owned()
}

/// Nombres de los PDFs de `dir`, ordenados, sin contar `excepto`.
fn pdfs_en(dir: &Path, excepto: &str) -> Vec<String> {
    let mut nombres: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".pdf") && n != excepto)
        .collect();
    nombres.sort();
    nombres
}

#[test]
fn diez_paginas_en_2_3_5_produce_tres_archivos() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);

    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5"])
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(
        pdfs_en(dir.path(), "doc.pdf"),
        ["doc_parte01_p1-2.pdf", "doc_parte02_p3-5.pdf", "doc_parte03_p6-10.pdf"]
    );
    verificar_parte(&dir.path().join("doc_parte01_p1-2.pdf"), 1, 2);
    verificar_parte(&dir.path().join("doc_parte02_p3-5.pdf"), 3, 5);
    verificar_parte(&dir.path().join("doc_parte03_p6-10.pdf"), 6, 10);

    let out = stdout(&salida);
    assert!(out.contains("parte 01: p1-2 ->"), "{out}");
    assert!(out.contains("parte 03: p6-10 ->"), "{out}");
}

#[test]
fn resto_incluir_por_defecto() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    pdfcorte().arg(&entrada).args(["--tamanos", "2,3,5"]).assert().success();
    assert_eq!(pdfs_en(dir.path(), "doc.pdf").len(), 4);
    verificar_parte(&dir.path().join("doc_parte04_p11-12.pdf"), 11, 12);
}

#[test]
fn resto_descartar() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5", "--resto", "descartar"])
        .assert()
        .success();
    assert_eq!(pdfs_en(dir.path(), "doc.pdf").len(), 3);
}

#[test]
fn resto_error_no_escribe_nada() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5", "--resto", "error"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&salida).contains("sobran 2 páginas"), "{}", stderr(&salida));
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());
}

#[test]
fn repetir_corta_cada_dos_paginas() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 7);
    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2", "--repetir"])
        .assert()
        .success();
    assert_eq!(
        pdfs_en(dir.path(), "doc.pdf"),
        [
            "doc_parte01_p1-2.pdf",
            "doc_parte02_p3-4.pdf",
            "doc_parte03_p5-6.pdf",
            "doc_parte04_p7-7.pdf"
        ]
    );
}

#[test]
fn exceso_falla_y_truncar_lo_recorta() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 8);

    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = stderr(&salida);
    assert!(err.contains("8 páginas") && err.contains("suman 10"), "{err}");
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());

    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5", "--truncar"])
        .assert()
        .success();
    verificar_parte(&dir.path().join("doc_parte03_p6-8.pdf"), 6, 8);
}

#[test]
fn simular_no_crea_archivos() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    let destino = dir.path().join("nuevo");

    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5", "--simular", "--salida"])
        .arg(&destino)
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(stdout(&salida).contains("parte 02: p3-5 ->"));
    assert!(!destino.exists(), "--simular no debe crear el directorio");
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());
}

#[test]
fn sin_sobrescribir_falla_si_la_salida_existe() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    let existente = dir.path().join("doc_parte02_p3-5.pdf");
    fs::write(&existente, "no tocar").unwrap();

    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5"])
        .assert()
        .failure()
        .get_output()
        .clone();
    assert!(stderr(&salida).contains("ya existe"), "{}", stderr(&salida));
    assert_eq!(fs::read_to_string(&existente).unwrap(), "no tocar");
    // No se escribe ninguna parte si alguna choca.
    assert!(!dir.path().join("doc_parte01_p1-2.pdf").exists());

    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2,3,5", "--sobrescribir"])
        .assert()
        .success();
    verificar_parte(&existente, 3, 5);
}

#[test]
fn salida_y_prefijo() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 4);
    let destino = dir.path().join("a").join("b");

    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "1,3", "--prefijo", "factura", "--salida"])
        .arg(&destino)
        .assert()
        .success();
    assert_eq!(
        pdfs_en(&destino, ""),
        ["factura_parte01_p1-1.pdf", "factura_parte02_p2-4.pdf"]
    );
}

#[test]
fn rechaza_prefijo_con_separador() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 4);
    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "2", "--prefijo", "../fuera"])
        .assert()
        .failure();
}

#[test]
fn rechaza_tamanos_invalidos() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    for tamanos in ["0", "-1", "2,-3", "abc", "", "2,,3", "1.5"] {
        let salida = pdfcorte()
            .arg(&entrada)
            .arg("--tamanos")
            .arg(tamanos)
            .assert()
            .failure()
            .get_output()
            .clone();
        let err = stderr(&salida);
        assert!(err.contains("tamaño inválido"), "«{tamanos}»: {err}");
        assert!(!err.contains("panicked"), "«{tamanos}»: {err}");
    }
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());
}

#[test]
fn tamanos_es_obligatorio() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 2);
    pdfcorte().arg(&entrada).assert().failure();
}

#[test]
fn archivos_invalidos_dan_error_legible() {
    let dir = TempDir::new().unwrap();
    let texto = dir.path().join("texto.pdf");
    fs::write(&texto, "hola").unwrap();
    let vacio = dir.path().join("vacio.pdf");
    fs::write(&vacio, b"").unwrap();
    let completo = fs::read(escribir_pdf(dir.path(), "completo.pdf", 10)).unwrap();
    let truncado = dir.path().join("truncado.pdf");
    fs::write(&truncado, &completo[..completo.len() / 3]).unwrap();

    let casos = [
        (texto, "no es un PDF"),
        (vacio, "no es un PDF"),
        (truncado, "no es un PDF válido o está dañado"),
        (dir.path().join("no-existe.pdf"), "no existe"),
    ];
    for (ruta, esperado) in casos {
        let salida = pdfcorte()
            .arg(&ruta)
            .args(["--tamanos", "1"])
            .assert()
            .failure()
            .get_output()
            .clone();
        let err = stderr(&salida);
        assert!(err.starts_with("error: "), "{}: {err}", ruta.display());
        assert!(err.contains(esperado), "{}: {err}", ruta.display());
        assert!(!err.contains("panicked"), "{}: {err}", ruta.display());
    }
}

#[test]
fn cortes_producen_partes_hasta_cada_pagina() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 60);
    pdfcorte()
        .arg(&entrada)
        .args(["--cortes", "22, 38,53"])
        .assert()
        .success();
    assert_eq!(
        pdfs_en(dir.path(), "doc.pdf"),
        [
            "doc_parte01_p1-22.pdf",
            "doc_parte02_p23-38.pdf",
            "doc_parte03_p39-53.pdf",
            "doc_parte04_p54-60.pdf"
        ]
    );
    verificar_parte(&dir.path().join("doc_parte01_p1-22.pdf"), 1, 22);
    verificar_parte(&dir.path().join("doc_parte02_p23-38.pdf"), 23, 38);
    verificar_parte(&dir.path().join("doc_parte03_p39-53.pdf"), 39, 53);
    verificar_parte(&dir.path().join("doc_parte04_p54-60.pdf"), 54, 60);
}

#[test]
fn cortes_con_simular_y_resto() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--cortes", "2,5,10", "--resto", "descartar", "--simular"])
        .assert()
        .success()
        .get_output()
        .clone();
    let out = stdout(&salida);
    assert!(out.contains("3 partes"), "{out}");
    assert!(out.contains("parte 03: p6-10 ->"), "{out}");
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());
}

#[test]
fn cortes_y_tamanos_son_excluyentes() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);

    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--cortes", "2", "--tamanos", "2"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    assert!(stderr(&salida).contains("cannot be used with"), "{}", stderr(&salida));

    let salida = pdfcorte().arg(&entrada).assert().code(2).get_output().clone();
    assert!(stderr(&salida).contains("required"), "{}", stderr(&salida));
}

#[test]
fn cortes_no_admiten_repetir() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--cortes", "2,5", "--repetir"])
        .assert()
        .code(2)
        .get_output()
        .clone();
    assert!(stderr(&salida).contains("cannot be used with"), "{}", stderr(&salida));
}

#[test]
fn cortes_invalidos() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);

    for cortes in ["", "0", "3,x", "4,,8"] {
        let salida = pdfcorte()
            .arg(&entrada)
            .args(["--cortes", cortes])
            .assert()
            .code(2)
            .get_output()
            .clone();
        assert!(stderr(&salida).contains("corte inválido"), "«{cortes}»: {}", stderr(&salida));
    }

    let casos = [
        ("5,9,7", "el corte 7 debe ser mayor que 9"),
        ("5,5", "el corte 5 debe ser mayor que 5"),
        ("5,20", "el corte 20 supera el total del PDF, que tiene 12 páginas"),
    ];
    for (cortes, esperado) in casos {
        let salida = pdfcorte()
            .arg(&entrada)
            .args(["--cortes", cortes])
            .assert()
            .code(1)
            .get_output()
            .clone();
        assert!(stderr(&salida).contains(esperado), "«{cortes}»: {}", stderr(&salida));
    }
    assert!(pdfs_en(dir.path(), "doc.pdf").is_empty());
}

#[test]
fn cortes_con_truncar() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 12);
    pdfcorte()
        .arg(&entrada)
        .args(["--cortes", "5,20,30", "--truncar"])
        .assert()
        .success();
    assert_eq!(
        pdfs_en(dir.path(), "doc.pdf"),
        ["doc_parte01_p1-5.pdf", "doc_parte02_p6-12.pdf"]
    );
    verificar_parte(&dir.path().join("doc_parte02_p6-12.pdf"), 6, 12);
}

#[test]
fn relleno_segun_numero_de_partes() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 101);
    pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "1", "--repetir", "--simular"])
        .assert()
        .success();
    let salida = pdfcorte()
        .arg(&entrada)
        .args(["--tamanos", "1", "--repetir"])
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(stdout(&salida).contains("parte 001: p1-1"));
    assert!(dir.path().join("doc_parte001_p1-1.pdf").exists());
    verificar_parte(&dir.path().join("doc_parte101_p101-101.pdf"), 101, 101);
}

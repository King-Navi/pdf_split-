mod common;

use std::fs;
use std::path::Path;

use lopdf::encryption::{EncryptionState, EncryptionVersion, Permissions};
use lopdf::{Document, Object, dictionary};
use pdfcorte::{ErrorCorte, Opciones, Rango, calcular_rangos, cargar_pdf, extraer_rango, guardar};
use tempfile::TempDir;

use common::{escribir_pdf, generar_pdf, texto_pagina, verificar_parte};

fn cortar(doc: &Document, rango: Rango, dir: &Path) -> std::path::PathBuf {
    let ruta = dir.join(format!("parte_{rango}.pdf"));
    guardar(&mut extraer_rango(doc, rango), &ruta).unwrap();
    ruta
}

#[test]
fn corta_10_paginas_en_2_3_5() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    let doc = cargar_pdf(&entrada).unwrap();

    let rangos = calcular_rangos(10, &[2, 3, 5], &Opciones::default()).unwrap();
    assert_eq!(rangos.len(), 3);
    for rango in rangos {
        let ruta = cortar(&doc, rango, dir.path());
        verificar_parte(&ruta, rango.inicio, rango.fin);
    }
}

#[test]
fn cada_pagina_por_separado() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 7);
    let doc = cargar_pdf(&entrada).unwrap();
    for n in 1..=7 {
        let ruta = cortar(&doc, Rango::new(n, n), dir.path());
        verificar_parte(&ruta, n, n);
    }
}

#[test]
fn conserva_recursos_y_mediabox_heredados() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 9);
    let doc = cargar_pdf(&entrada).unwrap();

    // La parte empieza a mitad de un nodo intermedio y cruza al siguiente.
    let ruta = cortar(&doc, Rango::new(3, 5), dir.path());
    let parte = Document::load(&ruta).unwrap();

    for (_, pagina_id) in parte.get_pages() {
        let pagina = parte.get_dictionary(pagina_id).unwrap();
        assert!(pagina.get(b"Resources").is_err(), "la página no debe tener recursos propios");
        assert!(pagina.get(b"MediaBox").is_err(), "la página no debe tener MediaBox propio");

        let fuentes = parte.get_page_fonts(pagina_id).unwrap();
        assert!(fuentes.contains_key(b"F1".as_slice()), "falta la fuente heredada");
        assert!(media_box_heredado(&parte, pagina_id).is_some(), "falta el MediaBox heredado");
    }
    verificar_parte(&ruta, 3, 5);
}

/// Busca `/MediaBox` subiendo por `/Parent`.
fn media_box_heredado(doc: &Document, mut id: lopdf::ObjectId) -> Option<Vec<Object>> {
    loop {
        let nodo = doc.get_dictionary(id).ok()?;
        if let Ok(caja) = nodo.get(b"MediaBox").and_then(Object::as_array) {
            return Some(caja.clone());
        }
        id = nodo.get(b"Parent").and_then(Object::as_reference).ok()?;
    }
}

#[test]
fn arbol_de_paginas_consistente() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    let doc = cargar_pdf(&entrada).unwrap();

    let parte = Document::load(cortar(&doc, Rango::new(4, 8), dir.path())).unwrap();
    let raiz_id = parte
        .catalog()
        .unwrap()
        .get(b"Pages")
        .and_then(Object::as_reference)
        .unwrap();
    assert_eq!(contar_hojas(&parte, raiz_id), 5);
}

/// Cuenta las hojas bajo `nodo` y comprueba que cada `/Count` coincide.
fn contar_hojas(doc: &Document, nodo: lopdf::ObjectId) -> i64 {
    let dict = doc.get_dictionary(nodo).unwrap();
    let Ok(hijos) = dict.get(b"Kids").and_then(Object::as_array) else {
        return 1;
    };
    let total = hijos
        .iter()
        .map(|h| contar_hojas(doc, h.as_reference().unwrap()))
        .sum();
    assert_eq!(dict.get(b"Count").and_then(Object::as_i64).unwrap(), total, "{nodo:?}");
    total
}

#[test]
fn sin_referencias_colgantes() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 10);
    let doc = cargar_pdf(&entrada).unwrap();

    for rango in [Rango::new(1, 1), Rango::new(4, 8), Rango::new(10, 10)] {
        let parte = Document::load(cortar(&doc, rango, dir.path())).unwrap();
        let mut refs = Vec::new();
        for objeto in parte.objects.values() {
            recoger(objeto, &mut refs);
        }
        for id in refs {
            assert!(parte.objects.contains_key(&id), "{rango}: referencia a {id:?} inexistente");
        }
    }
}

fn recoger(objeto: &Object, refs: &mut Vec<lopdf::ObjectId>) {
    match objeto {
        Object::Reference(id) => refs.push(*id),
        Object::Array(items) => items.iter().for_each(|o| recoger(o, refs)),
        Object::Dictionary(d) => d.iter().for_each(|(_, v)| recoger(v, refs)),
        Object::Stream(s) => s.dict.iter().for_each(|(_, v)| recoger(v, refs)),
        _ => {}
    }
}

#[test]
fn quita_marcadores_y_etiquetas() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 6);
    let doc = cargar_pdf(&entrada).unwrap();
    assert!(doc.catalog().unwrap().get(b"Outlines").is_ok());
    assert!(doc.catalog().unwrap().get(b"PageLabels").is_ok());

    let parte = Document::load(cortar(&doc, Rango::new(1, 2), dir.path())).unwrap();
    let catalogo = parte.catalog().unwrap();
    assert!(catalogo.get(b"Outlines").is_err());
    assert!(catalogo.get(b"PageLabels").is_err());
    assert!(catalogo.get(b"PageMode").is_err(), "/UseOutlines sin marcadores");
}

#[test]
fn open_action_solo_si_su_pagina_sigue() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 6);
    let doc = cargar_pdf(&entrada).unwrap();

    let con_pagina = Document::load(cortar(&doc, Rango::new(1, 3), dir.path())).unwrap();
    let destino = con_pagina
        .catalog()
        .unwrap()
        .get(b"OpenAction")
        .and_then(Object::as_array)
        .unwrap();
    let primera = *con_pagina.get_pages().get(&1).unwrap();
    assert_eq!(destino[0].as_reference().unwrap(), primera);

    let sin_pagina = Document::load(cortar(&doc, Rango::new(2, 3), dir.path())).unwrap();
    assert!(sin_pagina.catalog().unwrap().get(b"OpenAction").is_err());
}

#[test]
fn poda_paginas_borradas() {
    let dir = TempDir::new().unwrap();
    let entrada = escribir_pdf(dir.path(), "doc.pdf", 30);
    let doc = cargar_pdf(&entrada).unwrap();

    let ruta = cortar(&doc, Rango::new(1, 1), dir.path());
    let parte = Document::load(&ruta).unwrap();
    assert!(parte.objects.len() < doc.objects.len() / 4);
    assert!(fs::metadata(&ruta).unwrap().len() < fs::metadata(&entrada).unwrap().len());
}

#[test]
fn rechaza_texto_con_extension_pdf() {
    let dir = TempDir::new().unwrap();
    let ruta = dir.path().join("falso.pdf");
    fs::write(&ruta, "esto es solo texto, no un PDF\n").unwrap();
    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::NoEsPdf(_)), "{err:?}");
}

#[test]
fn rechaza_archivo_vacio() {
    let dir = TempDir::new().unwrap();
    let ruta = dir.path().join("vacio.pdf");
    fs::write(&ruta, b"").unwrap();
    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::NoEsPdf(_)), "{err:?}");
}

#[test]
fn rechaza_pdf_truncado() {
    let dir = TempDir::new().unwrap();
    let completo = escribir_pdf(dir.path(), "completo.pdf", 10);
    let bytes = fs::read(&completo).unwrap();
    let ruta = dir.path().join("truncado.pdf");
    fs::write(&ruta, &bytes[..bytes.len() / 3]).unwrap();

    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::PdfDanado { .. }), "{err:?}");
    assert!(err.to_string().contains("no es un PDF válido o está dañado"));
}

#[test]
fn rechaza_solo_cabecera() {
    let dir = TempDir::new().unwrap();
    let ruta = dir.path().join("cabecera.pdf");
    fs::write(&ruta, b"%PDF-1.7\n").unwrap();
    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::PdfDanado { .. }), "{err:?}");
}

#[test]
fn rechaza_archivo_inexistente() {
    let dir = TempDir::new().unwrap();
    let err = cargar_pdf(&dir.path().join("no-existe.pdf")).unwrap_err();
    assert!(matches!(err, ErrorCorte::NoExiste(_)), "{err:?}");
}

#[test]
fn rechaza_pdf_sin_paginas() {
    let mut doc = Document::with_version("1.5");
    let raiz_id = doc.add_object(dictionary! {
        "Type" => "Pages",
        "Kids" => Vec::<Object>::new(),
        "Count" => 0,
    });
    let catalogo_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => raiz_id });
    doc.trailer.set("Root", catalogo_id);

    let dir = TempDir::new().unwrap();
    let ruta = dir.path().join("sin-paginas.pdf");
    doc.save(&ruta).unwrap();
    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::SinPaginas(_)), "{err:?}");
}

fn pdf_cifrado(dir: &Path, nombre: &str, contrasena_usuario: &str) -> std::path::PathBuf {
    let mut doc = generar_pdf(4);
    doc.trailer.set(
        "ID",
        vec![
            Object::string_literal(b"0123456789abcdef".to_vec()),
            Object::string_literal(b"0123456789abcdef".to_vec()),
        ],
    );
    let estado = EncryptionState::try_from(EncryptionVersion::V2 {
        document: &doc,
        owner_password: "propietario",
        user_password: contrasena_usuario,
        key_length: 128,
        permissions: Permissions::all(),
    })
    .unwrap();
    doc.encrypt(&estado).unwrap();
    let ruta = dir.join(nombre);
    doc.save(&ruta).unwrap();
    ruta
}

#[test]
fn rechaza_pdf_con_contrasena() {
    let dir = TempDir::new().unwrap();
    let ruta = pdf_cifrado(dir.path(), "cifrado.pdf", "secreto");
    let err = cargar_pdf(&ruta).unwrap_err();
    assert!(matches!(err, ErrorCorte::Cifrado(_)), "{err:?}");
}

#[test]
fn abre_pdf_cifrado_sin_contrasena_de_usuario() {
    let dir = TempDir::new().unwrap();
    let ruta = pdf_cifrado(dir.path(), "cifrado.pdf", "");
    let doc = cargar_pdf(&ruta).unwrap();
    assert_eq!(texto_pagina(&doc, 2), "Pagina 2");

    let parte = cortar(&doc, Rango::new(2, 3), dir.path());
    verificar_parte(&parte, 2, 3);
}

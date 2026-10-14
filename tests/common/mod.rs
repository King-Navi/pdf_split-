//! Generación de PDFs de prueba con lopdf.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId, Stream, dictionary};

/// Páginas por nodo intermedio del árbol de páginas.
const PAGINAS_POR_NODO: u32 = 3;

/// Crea un documento de `paginas` páginas, cada una con el texto «Pagina N».
///
/// Para ejercitar la herencia, `Resources` y `MediaBox` solo están en el nodo
/// raíz `/Pages`, y las páginas cuelgan de nodos intermedios. El catálogo
/// incluye `/Outlines`, `/PageLabels`, `/PageMode` y un `/OpenAction` a la
/// página 1.
pub fn generar_pdf(paginas: u32) -> Document {
    let mut doc = Document::with_version("1.5");
    let raiz_id = doc.new_object_id();

    let fuente_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let recursos_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => fuente_id },
    });

    let mut nodos: Vec<Object> = Vec::new();
    let mut primera_pagina: Option<ObjectId> = None;
    let mut n = 1;
    while n <= paginas {
        let nodo_id = doc.new_object_id();
        let mut hijos: Vec<Object> = Vec::new();
        let fin = (n + PAGINAS_POR_NODO - 1).min(paginas);
        for pagina in n..=fin {
            let contenido = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![100.into(), 600.into()]),
                    Operation::new("Tj", vec![Object::string_literal(format!("Pagina {pagina}"))]),
                    Operation::new("ET", vec![]),
                ],
            };
            let bytes = contenido.encode().expect("codificar contenido");
            let contenido_id = doc.add_object(Stream::new(dictionary! {}, bytes));
            let pagina_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => nodo_id,
                "Contents" => contenido_id,
            });
            primera_pagina.get_or_insert(pagina_id);
            hijos.push(pagina_id.into());
        }
        doc.objects.insert(
            nodo_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Parent" => raiz_id,
                "Kids" => hijos,
                "Count" => i64::from(fin - n + 1),
            }),
        );
        nodos.push(nodo_id.into());
        n = fin + 1;
    }

    doc.objects.insert(
        raiz_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => nodos,
            "Count" => i64::from(paginas),
            "Resources" => recursos_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        }),
    );

    let mut catalogo = dictionary! {
        "Type" => "Catalog",
        "Pages" => raiz_id,
        // Etiquetas en romanas que no deben influir en el conteo físico.
        "PageLabels" => dictionary! {
            "Nums" => vec![0.into(), Object::Dictionary(dictionary! { "S" => "r" })],
        },
    };
    if let Some(primera) = primera_pagina {
        let marcadores_id = doc.new_object_id();
        let marcador_id = doc.add_object(dictionary! {
            "Title" => Object::string_literal("Inicio"),
            "Parent" => marcadores_id,
            "Dest" => vec![primera.into(), "Fit".into()],
        });
        doc.objects.insert(
            marcadores_id,
            Object::Dictionary(dictionary! {
                "Type" => "Outlines",
                "First" => marcador_id,
                "Last" => marcador_id,
                "Count" => 1,
            }),
        );
        catalogo.set("Outlines", marcadores_id);
        catalogo.set("PageMode", "UseOutlines");
        catalogo.set("OpenAction", vec![primera.into(), "Fit".into()]);
    }
    let catalogo_id = doc.add_object(catalogo);
    doc.trailer.set("Root", catalogo_id);
    doc
}

/// Escribe en `dir/nombre` un PDF de `paginas` páginas y devuelve su ruta.
pub fn escribir_pdf(dir: &Path, nombre: &str, paginas: u32) -> PathBuf {
    let ruta = dir.join(nombre);
    generar_pdf(paginas).save(&ruta).expect("guardar PDF de prueba");
    ruta
}

/// Texto de la página física `n` (1-indexada), sin espacios alrededor.
pub fn texto_pagina(doc: &Document, n: u32) -> String {
    doc.extract_text(&[n])
        .expect("extraer texto")
        .trim()
        .to_string()
}

/// Comprueba que `ruta` es un PDF con las páginas `inicio..=fin` del original.
pub fn verificar_parte(ruta: &Path, inicio: u32, fin: u32) {
    let doc = Document::load(ruta).expect("abrir parte");
    let paginas = fin - inicio + 1;
    assert_eq!(doc.get_pages().len() as u32, paginas, "{}", ruta.display());
    assert_eq!(texto_pagina(&doc, 1), format!("Pagina {inicio}"), "{}", ruta.display());
    assert_eq!(texto_pagina(&doc, paginas), format!("Pagina {fin}"), "{}", ruta.display());
}

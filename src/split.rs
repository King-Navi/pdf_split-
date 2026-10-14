//! Carga, validación y extracción de páginas con lopdf.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::error::ErrorCorte;
use crate::plan::Rango;

/// La especificación permite basura antes de la cabecera, dentro del primer KiB.
const LIMITE_CABECERA: usize = 1024;
const CABECERA: &[u8] = b"%PDF-";

/// Lee y valida un PDF: debe existir, tener la cabecera `%PDF-`, poder
/// cargarse, abrirse sin contraseña y tener al menos una página.
pub fn cargar_pdf(ruta: &Path) -> Result<Document, ErrorCorte> {
    let bytes = fs::read(ruta).map_err(|fuente| match fuente.kind() {
        io::ErrorKind::NotFound => ErrorCorte::NoExiste(ruta.to_path_buf()),
        _ => ErrorCorte::Lectura {
            ruta: ruta.to_path_buf(),
            fuente,
        },
    })?;

    let inicio = &bytes[..bytes.len().min(LIMITE_CABECERA)];
    if !inicio.windows(CABECERA.len()).any(|w| w == CABECERA) {
        return Err(ErrorCorte::NoEsPdf(ruta.to_path_buf()));
    }

    let doc = Document::load_mem(&bytes).map_err(|fuente| match fuente {
        lopdf::Error::Decryption(_) | lopdf::Error::InvalidPassword => {
            ErrorCorte::Cifrado(ruta.to_path_buf())
        }
        _ => ErrorCorte::PdfDanado {
            ruta: ruta.to_path_buf(),
            fuente,
        },
    })?;

    // lopdf descifra solo si la contraseña de usuario vacía funciona; si no,
    // carga el documento pero deja `/Encrypt` en el trailer.
    if doc.is_encrypted() {
        return Err(ErrorCorte::Cifrado(ruta.to_path_buf()));
    }
    if doc.get_pages().is_empty() {
        return Err(ErrorCorte::SinPaginas(ruta.to_path_buf()));
    }
    Ok(doc)
}

/// Devuelve una copia de `doc` con solo las páginas físicas de `rango`.
///
/// Clona el documento completo, así que el pico de memoria es de unas dos
/// veces el documento cargado.
pub fn extraer_rango(doc: &Document, rango: Rango) -> Document {
    let mut parte = doc.clone();

    let fuera: HashSet<ObjectId> = parte
        .get_pages()
        .into_iter()
        .filter(|&(n, _)| !rango.contiene(n))
        .map(|(_, id)| id)
        .collect();
    let abre_en_borrada = pagina_inicial(&parte).is_some_and(|id| fuera.contains(&id));
    borrar_paginas(&mut parte, &fuera);

    // Los marcadores y etiquetas pueden apuntar a páginas borradas o mostrar
    // una numeración que ya no corresponde. Se quitan antes de podar para que
    // sus objetos también se eliminen.
    if let Ok(catalogo) = parte.catalog_mut() {
        catalogo.remove(b"Outlines");
        catalogo.remove(b"PageLabels");
        if catalogo
            .get(b"PageMode")
            .and_then(Object::as_name)
            .is_ok_and(|modo| modo == b"UseOutlines")
        {
            catalogo.remove(b"PageMode");
        }
        // Sin su página, el destino quedaría como `[/Fit]`, que no es válido.
        if abre_en_borrada {
            catalogo.remove(b"OpenAction");
        }
    }

    podar(&mut parte);
    parte.compress();
    parte
}

/// Página a la que salta `/OpenAction`, sea un destino explícito o una acción
/// `GoTo`. `None` si no hay o es un destino con nombre.
fn pagina_inicial(doc: &Document) -> Option<ObjectId> {
    let catalogo = doc.catalog().ok()?;
    let (_, accion) = doc.dereference(catalogo.get(b"OpenAction").ok()?).ok()?;
    let destino = match accion {
        Object::Dictionary(dict) => doc.dereference(dict.get(b"D").ok()?).ok()?.1,
        otro => otro,
    };
    destino.as_array().ok()?.first()?.as_reference().ok()
}

// `Document::delete_pages` y `Document::prune_objects` hacen lo mismo que las
// dos funciones siguientes, pero recorren el grafo con búsquedas lineales en un
// `Vec`, y `delete_pages` lo recorre una vez por cada página borrada. Cortar un
// PDF de 500 páginas en partes de 1 tardaba ~30 s; así es lineal por parte.

/// Borra las páginas `fuera`, quita toda referencia a ellas (de `/Kids`,
/// destinos, etc.) y recalcula `/Count` en el árbol de páginas. Los atributos
/// heredables (`Resources`, `MediaBox`, `Rotate`...) de los nodos padres no se
/// tocan.
fn borrar_paginas(doc: &mut Document, fuera: &HashSet<ObjectId>) {
    if fuera.is_empty() {
        return;
    }
    doc.objects.retain(|id, _| !fuera.contains(id));
    for objeto in doc.objects.values_mut() {
        quitar_referencias(objeto, fuera);
    }

    let raiz = doc
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(Object::as_reference);
    if let Ok(raiz) = raiz {
        recontar(doc, raiz, &mut HashSet::new());
    }
}

fn quitar_referencias(objeto: &mut Object, fuera: &HashSet<ObjectId>) {
    match objeto {
        Object::Array(items) => {
            items.retain(|item| !apunta_a(item, fuera));
            for item in items {
                quitar_referencias(item, fuera);
            }
        }
        Object::Dictionary(dict) => quitar_referencias_dict(dict, fuera),
        Object::Stream(stream) => quitar_referencias_dict(&mut stream.dict, fuera),
        _ => {}
    }
}

fn quitar_referencias_dict(dict: &mut Dictionary, fuera: &HashSet<ObjectId>) {
    let claves: Vec<Vec<u8>> = dict
        .iter()
        .filter(|(_, v)| apunta_a(v, fuera))
        .map(|(k, _)| k.clone())
        .collect();
    for clave in claves {
        dict.remove(&clave);
    }
    for (_, valor) in dict.iter_mut() {
        quitar_referencias(valor, fuera);
    }
}

fn apunta_a(objeto: &Object, ids: &HashSet<ObjectId>) -> bool {
    matches!(objeto, Object::Reference(id) if ids.contains(id))
}

/// Recalcula `/Count` bajo `nodo` y devuelve cuántas páginas contiene.
fn recontar(doc: &mut Document, nodo: ObjectId, visitados: &mut HashSet<ObjectId>) -> i64 {
    // Protege contra ciclos en PDFs malformados.
    if !visitados.insert(nodo) {
        return 0;
    }
    let Ok(dict) = doc.get_dictionary(nodo) else {
        return 0;
    };
    let Ok(hijos) = dict.get(b"Kids").and_then(Object::as_array) else {
        return 1; // Hoja: una página.
    };
    let hijos: Vec<ObjectId> = hijos.iter().filter_map(|h| h.as_reference().ok()).collect();

    let total = hijos.into_iter().map(|h| recontar(doc, h, visitados)).sum();
    if let Ok(dict) = doc.get_dictionary_mut(nodo) {
        dict.set("Count", total);
    }
    total
}

/// Elimina los objetos que no son alcanzables desde el trailer.
fn podar(doc: &mut Document) {
    let mut pendientes = Vec::new();
    for (_, valor) in doc.trailer.iter() {
        recoger_referencias(valor, &mut pendientes);
    }
    let mut alcanzables = HashSet::new();
    while let Some(id) = pendientes.pop() {
        if alcanzables.insert(id)
            && let Some(objeto) = doc.objects.get(&id)
        {
            recoger_referencias(objeto, &mut pendientes);
        }
    }
    doc.objects.retain(|id, _| alcanzables.contains(id));
}

fn recoger_referencias(objeto: &Object, pendientes: &mut Vec<ObjectId>) {
    match objeto {
        Object::Reference(id) => pendientes.push(*id),
        Object::Array(items) => items.iter().for_each(|o| recoger_referencias(o, pendientes)),
        Object::Dictionary(dict) => dict.iter().for_each(|(_, v)| recoger_referencias(v, pendientes)),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .for_each(|(_, v)| recoger_referencias(v, pendientes)),
        _ => {}
    }
}

/// Guarda `doc` en `ruta` con flujos de objetos y de referencias cruzadas
/// (PDF 1.5). Sin ellos, los objetos pequeños, como los de un árbol de
/// estructura de un PDF etiquetado, se escriben sin comprimir en cada parte.
pub fn guardar(doc: &mut Document, ruta: &Path) -> Result<(), ErrorCorte> {
    let error = |fuente| ErrorCorte::Escritura {
        ruta: ruta.to_path_buf(),
        fuente,
    };
    let mut archivo = BufWriter::new(File::create(ruta).map_err(error)?);
    doc.save_modern(&mut archivo).map_err(error)?;
    archivo.flush().map_err(error)
}

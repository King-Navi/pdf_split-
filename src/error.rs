use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errores del dominio de `pdfcorte`.
#[derive(Debug, Error)]
pub enum ErrorCorte {
    #[error("la lista de tamaños está vacía")]
    TamanosVacios,

    #[error("tamaño inválido «{0}»: debe ser un entero mayor que 0")]
    TamanoInvalido(String),

    #[error("la lista de cortes está vacía")]
    CortesVacios,

    #[error("corte inválido «{0}»: debe ser un entero mayor que 0")]
    CorteInvalido(String),

    #[error("el corte {corte} debe ser mayor que {anterior}")]
    CorteNoCreciente { anterior: u32, corte: u32 },

    #[error(
        "el corte {corte} supera el total del PDF, que tiene {}; \
         use --truncar para recortarlo",
        paginas(*total)
    )]
    CorteExcedeTotal { corte: u32, total: u32 },

    #[error("el documento no tiene páginas")]
    DocumentoVacio,

    #[error(
        "el PDF tiene {}, pero los tamaños suman {pedidas}; \
         use --truncar para recortar el último bloque",
        paginas(*total)
    )]
    PaginasInsuficientes { total: u32, pedidas: u64 },

    #[error(
        "{} {} (p{inicio}-{fin}) y se indicó --resto error",
        if fin == inicio { "sobra" } else { "sobran" },
        paginas(fin - inicio + 1)
    )]
    PaginasSobrantes { inicio: u32, fin: u32 },

    #[error("con estas opciones no se genera ninguna parte")]
    SinPartes,

    #[error("el prefijo «{0}» no es válido: no puede estar vacío ni contener separadores de ruta")]
    PrefijoInvalido(String),

    #[error("el archivo «{0}» no existe")]
    NoExiste(PathBuf),

    #[error("no se puede leer «{ruta}»")]
    Lectura {
        ruta: PathBuf,
        #[source]
        fuente: io::Error,
    },

    #[error("«{0}» no es un PDF: no contiene la cabecera %PDF-")]
    NoEsPdf(PathBuf),

    #[error("«{ruta}» no es un PDF válido o está dañado")]
    PdfDanado {
        ruta: PathBuf,
        #[source]
        fuente: lopdf::Error,
    },

    #[error(
        "«{0}» está cifrado y no se puede abrir sin contraseña; \
         los PDFs con contraseña no están soportados"
    )]
    Cifrado(PathBuf),

    #[error("«{0}» no tiene páginas")]
    SinPaginas(PathBuf),

    #[error("el archivo de salida «{0}» ya existe; use --sobrescribir para reemplazarlo")]
    SalidaExiste(PathBuf),

    #[error("no se pudo crear el directorio «{ruta}»")]
    CrearDirectorio {
        ruta: PathBuf,
        #[source]
        fuente: io::Error,
    },

    #[error("no se pudo escribir «{ruta}»")]
    Escritura {
        ruta: PathBuf,
        #[source]
        fuente: io::Error,
    },
}

/// «1 página», «N páginas».
pub fn paginas(n: u32) -> String {
    if n == 1 {
        "1 página".to_string()
    } else {
        format!("{n} páginas")
    }
}

use std::path::PathBuf;

use clap::{ArgGroup, Parser};
use pdfcorte::{Resto, parsear_corte, parsear_tamano};

/// Separa un PDF en bloques de páginas físicas consecutivas.
///
/// Las páginas se cuentan desde la 1 en el orden del documento; nunca se usan
/// las etiquetas de página del PDF.
#[derive(Debug, Parser)]
#[command(name = "pdfcorte", version)]
#[command(group(
    ArgGroup::new("modo")
        .args(["tamanos", "cortes"])
        .required(true)
        .multiple(false)
))]
pub struct Args {
    /// PDF de entrada.
    #[arg(value_name = "ENTRADA.pdf")]
    pub entrada: PathBuf,

    /// Tamaños de los bloques, separados por comas (por ejemplo 2,3,5).
    #[arg(
        long,
        value_delimiter = ',',
        allow_hyphen_values = true,
        value_parser = tamano
    )]
    pub tamanos: Option<Vec<u32>>,

    /// Última página de cada parte, separadas por comas (por ejemplo 22,38,53).
    #[arg(
        long,
        value_delimiter = ',',
        allow_hyphen_values = true,
        value_parser = corte
    )]
    pub cortes: Option<Vec<u32>>,

    /// Qué hacer con las páginas que sobran después del último bloque o corte.
    #[arg(long, value_enum, default_value_t = Resto::Incluir)]
    pub resto: Resto,

    /// Recorre los tamaños de forma cíclica hasta agotar el documento.
    #[arg(long, conflicts_with = "cortes")]
    pub repetir: bool,

    /// Si los tamaños o cortes superan el total de páginas, recorta el último
    /// bloque en lugar de fallar.
    #[arg(long)]
    pub truncar: bool,

    /// Directorio de salida. Por defecto, el directorio del archivo de entrada.
    #[arg(long, value_name = "DIR")]
    pub salida: Option<PathBuf>,

    /// Prefijo de los archivos generados. Por defecto, el nombre de la entrada
    /// sin extensión.
    #[arg(long, value_name = "NOMBRE")]
    pub prefijo: Option<String>,

    /// Reemplaza los archivos de salida que ya existan.
    #[arg(long)]
    pub sobrescribir: bool,

    /// Solo muestra el plan de cortes, sin escribir archivos.
    #[arg(long)]
    pub simular: bool,
}

fn tamano(texto: &str) -> Result<u32, String> {
    parsear_tamano(texto).map_err(|e| e.to_string())
}

fn corte(texto: &str) -> Result<u32, String> {
    parsear_corte(texto).map_err(|e| e.to_string())
}

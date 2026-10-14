//! `pdfcorte`: separa un PDF en bloques de páginas físicas consecutivas.

pub mod error;
pub mod plan;
pub mod split;

pub use error::ErrorCorte;
pub use plan::{
    Opciones, Rango, Resto, calcular_rangos, calcular_rangos_por_cortes, cortes_a_tamanos,
    nombre_parte, parsear_corte, parsear_tamano,
};
pub use split::{cargar_pdf, extraer_rango, guardar};

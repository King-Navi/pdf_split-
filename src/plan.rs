//! Lógica pura de cortes: decide qué rangos de páginas físicas forman cada
//! parte. No lee ni escribe PDFs.

use std::fmt;

use crate::error::ErrorCorte;

/// Rango de páginas físicas, 1-indexado e inclusivo en ambos extremos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rango {
    pub inicio: u32,
    pub fin: u32,
}

impl Rango {
    pub fn new(inicio: u32, fin: u32) -> Self {
        Self { inicio, fin }
    }

    pub fn paginas(&self) -> u32 {
        self.fin - self.inicio + 1
    }

    pub fn contiene(&self, pagina: u32) -> bool {
        (self.inicio..=self.fin).contains(&pagina)
    }
}

impl fmt::Display for Rango {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}-{}", self.inicio, self.fin)
    }
}

/// Qué hacer con las páginas que no cubre ningún bloque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, clap::ValueEnum)]
pub enum Resto {
    /// Genera una parte final con las páginas sobrantes.
    #[default]
    Incluir,
    /// Ignora las páginas sobrantes.
    Descartar,
    /// Falla si sobran páginas.
    Error,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Opciones {
    pub resto: Resto,
    /// Recorre los tamaños de forma cíclica hasta agotar el documento.
    pub repetir: bool,
    /// Recorta el último bloque si los tamaños superan el total de páginas.
    pub truncar: bool,
}

/// Interpreta un tamaño de bloque: entero mayor que 0.
pub fn parsear_tamano(texto: &str) -> Result<u32, ErrorCorte> {
    match texto.trim().parse::<u32>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(ErrorCorte::TamanoInvalido(texto.to_string())),
    }
}

/// Calcula los rangos de cada parte sobre un documento de `total` páginas.
///
/// Sin `repetir`, los bloques se aplican una sola vez; si suman más que
/// `total` se falla, salvo con `truncar`, que recorta el bloque que cruza el
/// final y descarta los siguientes. Las páginas que queden después se tratan
/// según `resto`.
///
/// Con `repetir`, los tamaños se recorren cíclicamente; el bloque final
/// incompleto, si lo hay, es el resto y se trata según `resto`.
pub fn calcular_rangos(
    total: u32,
    tamanos: &[u32],
    opciones: &Opciones,
) -> Result<Vec<Rango>, ErrorCorte> {
    if tamanos.is_empty() {
        return Err(ErrorCorte::TamanosVacios);
    }
    if tamanos.contains(&0) {
        return Err(ErrorCorte::TamanoInvalido("0".to_string()));
    }
    if total == 0 {
        return Err(ErrorCorte::DocumentoVacio);
    }

    let mut rangos = Vec::new();
    // Primera página aún no asignada. u64 para que la suma nunca desborde.
    let mut inicio: u64 = 1;
    let total64 = u64::from(total);

    if opciones.repetir {
        for &tamano in tamanos.iter().cycle() {
            let fin = inicio + u64::from(tamano) - 1;
            if fin > total64 {
                break;
            }
            rangos.push(rango(inicio, fin));
            inicio = fin + 1;
            if inicio > total64 {
                break;
            }
        }
    } else {
        let pedidas: u64 = tamanos.iter().map(|&t| u64::from(t)).sum();
        if pedidas > total64 && !opciones.truncar {
            return Err(ErrorCorte::PaginasInsuficientes { total, pedidas });
        }
        for &tamano in tamanos {
            if inicio > total64 {
                break;
            }
            let fin = (inicio + u64::from(tamano) - 1).min(total64);
            rangos.push(rango(inicio, fin));
            inicio = fin + 1;
        }
    }

    if inicio <= total64 {
        let sobrante = rango(inicio, total64);
        match opciones.resto {
            Resto::Incluir => rangos.push(sobrante),
            Resto::Descartar => {}
            Resto::Error => {
                return Err(ErrorCorte::PaginasSobrantes {
                    inicio: sobrante.inicio,
                    fin: sobrante.fin,
                });
            }
        }
    }

    if rangos.is_empty() {
        return Err(ErrorCorte::SinPartes);
    }
    Ok(rangos)
}

/// Interpreta un corte (última página de una parte): entero mayor que 0.
pub fn parsear_corte(texto: &str) -> Result<u32, ErrorCorte> {
    parsear_tamano(texto).map_err(|_| ErrorCorte::CorteInvalido(texto.to_string()))
}

/// Convierte cortes (última página de cada parte) en tamaños de bloque:
/// `[22, 38, 53]` -> `[22, 16, 15]`. Los cortes deben ser mayores que 0 y
/// estrictamente crecientes.
pub fn cortes_a_tamanos(cortes: &[u32]) -> Result<Vec<u32>, ErrorCorte> {
    validar_cortes(cortes)?;
    let anteriores = std::iter::once(0).chain(cortes.iter().copied());
    Ok(cortes.iter().zip(anteriores).map(|(c, a)| c - a).collect())
}

/// Calcula los rangos a partir de cortes. Las páginas después del último corte
/// se tratan según `opciones.resto`; `opciones.repetir` se ignora.
///
/// Un corte mayor que `total` es un error, salvo con `opciones.truncar`, que lo
/// recorta a `total` y descarta los cortes posteriores. Esto se valida antes de
/// convertir a tamaños para que los errores hablen de cortes.
pub fn calcular_rangos_por_cortes(
    total: u32,
    cortes: &[u32],
    opciones: &Opciones,
) -> Result<Vec<Rango>, ErrorCorte> {
    validar_cortes(cortes)?;
    if total == 0 {
        return Err(ErrorCorte::DocumentoVacio);
    }

    let mut ajustados = Vec::with_capacity(cortes.len());
    for &corte in cortes {
        if corte <= total {
            ajustados.push(corte);
            continue;
        }
        if !opciones.truncar {
            return Err(ErrorCorte::CorteExcedeTotal { corte, total });
        }
        // Si el corte anterior ya era la última página, no queda nada que recortar.
        if ajustados.last() != Some(&total) {
            ajustados.push(total);
        }
        break;
    }

    let tamanos = cortes_a_tamanos(&ajustados)?;
    let opciones = Opciones {
        repetir: false,
        ..*opciones
    };
    calcular_rangos(total, &tamanos, &opciones)
}

fn validar_cortes(cortes: &[u32]) -> Result<(), ErrorCorte> {
    if cortes.is_empty() {
        return Err(ErrorCorte::CortesVacios);
    }
    if cortes.contains(&0) {
        return Err(ErrorCorte::CorteInvalido("0".to_string()));
    }
    if let Some(par) = cortes.windows(2).find(|par| par[1] <= par[0]) {
        return Err(ErrorCorte::CorteNoCreciente {
            anterior: par[0],
            corte: par[1],
        });
    }
    Ok(())
}

/// Nombre del archivo de una parte: `{prefijo}_parte{NN}_p{inicio}-{fin}.pdf`.
/// `indice` es 1-indexado; `NN` usa tantos dígitos como `total_partes`
/// (mínimo 2).
pub fn nombre_parte(prefijo: &str, indice: usize, total_partes: usize, rango: Rango) -> String {
    let ancho = total_partes.to_string().len().max(2);
    format!("{prefijo}_parte{indice:0ancho$}_{rango}.pdf")
}

// Solo se llama con valores acotados por `total: u32`.
fn rango(inicio: u64, fin: u64) -> Rango {
    Rango::new(inicio as u32, fin as u32)
}

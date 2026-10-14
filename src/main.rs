mod cli;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use pdfcorte::error::paginas;
use pdfcorte::{
    ErrorCorte, Opciones, Rango, calcular_rangos, calcular_rangos_por_cortes, cargar_pdf,
    extraer_rango, guardar, nombre_parte,
};

use crate::cli::Args;

fn main() -> ExitCode {
    let args = Args::parse();
    match ejecutar(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn ejecutar(args: &Args) -> Result<()> {
    let prefijo = prefijo(args)?;
    let directorio = directorio_salida(args);

    let doc = cargar_pdf(&args.entrada)?;
    // `cargar_pdf` garantiza al menos una página.
    let total = u32::try_from(doc.get_pages().len())?;

    let opciones = Opciones {
        resto: args.resto,
        repetir: args.repetir,
        truncar: args.truncar,
    };
    // clap exige exactamente uno de los dos; sin ninguno, `calcular_rangos`
    // falla con «lista de tamaños vacía» en vez de entrar en pánico.
    let rangos = match &args.cortes {
        Some(cortes) => calcular_rangos_por_cortes(total, cortes, &opciones)?,
        None => calcular_rangos(total, args.tamanos.as_deref().unwrap_or_default(), &opciones)?,
    };

    let partes: Vec<(Rango, PathBuf)> = rangos
        .iter()
        .enumerate()
        .map(|(i, &rango)| {
            let nombre = nombre_parte(&prefijo, i + 1, rangos.len(), rango);
            (rango, directorio.join(nombre))
        })
        .collect();

    // Se comprueba todo antes de escribir nada, para no dejar salidas a medias.
    if !args.simular
        && !args.sobrescribir
        && let Some((_, ruta)) = partes.iter().find(|(_, ruta)| ruta.exists())
    {
        return Err(ErrorCorte::SalidaExiste(ruta.clone()).into());
    }

    println!(
        "{}: {}, {} {}{}",
        args.entrada.display(),
        paginas(total),
        partes.len(),
        if partes.len() == 1 { "parte" } else { "partes" },
        if args.simular { " (simulación, no se escriben archivos)" } else { "" }
    );
    let ancho = partes.len().to_string().len().max(2);
    for (i, (rango, ruta)) in partes.iter().enumerate() {
        println!("parte {:0ancho$}: {rango} -> {}", i + 1, ruta.display());
    }

    if args.simular {
        return Ok(());
    }

    fs::create_dir_all(&directorio).map_err(|fuente| ErrorCorte::CrearDirectorio {
        ruta: directorio.clone(),
        fuente,
    })?;

    for (rango, ruta) in &partes {
        let mut parte = extraer_rango(&doc, *rango);
        guardar(&mut parte, ruta)?;
    }
    Ok(())
}

fn prefijo(args: &Args) -> Result<String, ErrorCorte> {
    match &args.prefijo {
        Some(p) => {
            if p.is_empty() || p.contains(['/', '\\']) || p == "." || p == ".." {
                Err(ErrorCorte::PrefijoInvalido(p.clone()))
            } else {
                Ok(p.clone())
            }
        }
        None => Ok(args
            .entrada
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "documento".to_string())),
    }
}

/// Para una entrada sin directorio («doc.pdf») devuelve la ruta vacía, que
/// equivale al directorio actual.
fn directorio_salida(args: &Args) -> PathBuf {
    match &args.salida {
        Some(dir) => dir.clone(),
        None => args.entrada.parent().map(Path::to_path_buf).unwrap_or_default(),
    }
}

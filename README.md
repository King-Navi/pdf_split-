# pdfcorte

Separa un PDF en varios PDFs por bloques de páginas. Las páginas se cuentan
físicamente desde la 1 (se ignoran las etiquetas del documento).

## Instalación

```sh
cargo install --path .
```

## Uso

Indica **tamaños** de bloque o **cortes** (última página de cada parte), no ambos:

```sh
pdfcorte factura.pdf --tamanos 2,3,5    # p1-2, p3-5, p6-10
pdfcorte libro.pdf --cortes 22,38,53    # p1-22, p23-38, p39-53
```

Los archivos se llaman `{prefijo}_parte{NN}_p{inicio}-{fin}.pdf`, por ejemplo
`factura_parte01_p1-2.pdf`.

## Opciones

| Opción | Efecto |
|---|---|
| `--tamanos 2,3,5` | Tamaños de los bloques (enteros > 0). |
| `--cortes 22,38,53` | Última página de cada parte (crecientes). Con espacios, entre comillas: `"22, 38"`. |
| `--resto incluir` | Las páginas sobrantes forman una parte final (por defecto). |
| `--resto descartar` | Ignora las páginas sobrantes. |
| `--resto error` | Falla si sobran páginas. |
| `--repetir` | Repite los tamaños hasta terminar el documento (no con `--cortes`). |
| `--truncar` | Si los tamaños o cortes superan el total de páginas, recorta el último bloque en vez de fallar. |
| `--salida DIR` | Directorio de salida (por defecto, el del PDF). Se crea si no existe. |
| `--prefijo NOMBRE` | Prefijo de los archivos (por defecto, el nombre del PDF). |
| `--sobrescribir` | Reemplaza archivos existentes (sin ella, falla). |
| `--simular` | Muestra los cortes sin escribir nada. |

## Ejemplos

```sh
pdfcorte libro.pdf --tamanos 2 --repetir             # cortar cada 2 páginas
pdfcorte libro.pdf --tamanos 10 --simular            # ver el plan antes de cortar
pdfcorte libro.pdf --tamanos 4,4,4 --truncar         # 10 páginas -> p1-4, p5-8, p9-10
pdfcorte libro.pdf --tamanos 5 --repetir --resto descartar   # solo bloques completos
pdfcorte libro.pdf --tamanos 3,7 --salida cortes/ --prefijo cap
pdfcorte libro.pdf --cortes 22,38,53 --resto descartar   # sin la parte p54-fin
pdfcorte libro.pdf --cortes 22,200 --truncar             # 179 páginas -> p1-22, p23-179
```

## Notas

- Solo acepta PDFs válidos; los PDFs con contraseña no están soportados.
- Cada parte pierde los marcadores y las etiquetas de página del original.
- El PDF se carga completo en memoria, así que con archivos muy grandes puede consumir mucha.

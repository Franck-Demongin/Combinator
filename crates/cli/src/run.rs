use anyhow::{Context, Result, bail};
use clap::Parser;
use combinator_core::{SourceDimensions, SourceError, compose};
use image::{DynamicImage, ImageBuffer, ImageReader, RgbImage, codecs::jpeg::JpegEncoder};
use lopdf::{Document, Object, Stream, content::Content, content::Operation};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

#[derive(Parser, Debug)]
#[command(
    name = "combinator",
    version,
    about = "Composition de sources (images, PDF, HEIC) sur planches A4"
)]
struct Cli {
    /// Fichiers ou dossiers sources, en complement de --dir.
    #[arg(long = "source", short = 's', value_name = "PATH")]
    source: Vec<PathBuf>,

    /// Repertoire source unique (niveau unique, pas de sous-dossiers).
    #[arg(long = "dir", short = 'd', value_name = "DIR")]
    dir: Option<PathBuf>,

    /// Entrees positionnelles (fichiers ou repertoires).
    #[arg(value_name = "INPUT")]
    inputs: Vec<PathBuf>,

    /// Chemin du PDF final genere.
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    output: Option<PathBuf>,

    /// Chemin d'un rapport JSON complementaire.
    #[arg(long = "report", value_name = "PATH")]
    report: Option<PathBuf>,

    /// Ecrase un rapport existant.
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// Affiche le plan sans ecrire le PDF.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Sortie silencieuse en cas de succes.
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

#[derive(Clone)]
struct Raster {
    path: String,
    page_index: usize,
    width: u32,
    height: u32,
    dpi_x: f32,
    dpi_y: f32,
    rgb: Vec<u8>,
}

impl Raster {
    fn dimensions_mm(&self) -> SourceDimensions {
        let width_mm = self.width as f64 / self.dpi_x as f64 * 25.4;
        let height_mm = self.height as f64 / self.dpi_y as f64 * 25.4;
        SourceDimensions::new(width_mm, height_mm).expect("raster dimensions are valid")
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    let mut input_paths: Vec<PathBuf> = Vec::new();
    input_paths.extend(cli.source.clone());
    if let Some(dir) = cli.dir.clone() {
        input_paths.push(dir);
    }
    input_paths.extend(cli.inputs.clone());

    if input_paths.is_empty() {
        bail!("aucune source fournie (utilisez --source, --dir ou des arguments positionnels)");
    }

    let mut files: Vec<PathBuf> = Vec::new();
    for path in &input_paths {
        if path.is_dir() {
            files.extend(expand_directory(path).map_err(source_error_to_anyhow)?);
        } else if path.is_file() {
            if !is_supported_path(path) {
                bail!("format non supporte : {}", path.display());
            }
            files.push(path.clone());
        } else {
            bail!("chemin introuvable : {}", path.display());
        }
    }

    files.sort_by(|left, right| {
        natord::compare(
            left.to_string_lossy().as_ref(),
            right.to_string_lossy().as_ref(),
        )
    });

    if files.is_empty() {
        bail!("aucun fichier source supporte trouve");
    }

    let mut rasters: Vec<Raster> = Vec::new();
    for file in &files {
        match file_extension_lower(file).as_str() {
            "png" | "jpg" | "jpeg" => rasters.push(raster_from_image(file)?),
            "heic" | "heif" => rasters.push(raster_from_heic(file)?),
            "pdf" => rasters.extend(raster_from_pdf(file)?),
            _ => bail!("format non supporte : {}", file.display()),
        }
    }

    if rasters.is_empty() {
        bail!("aucune source valide n'a pu etre lue");
    }

    let dimensions: Vec<SourceDimensions> = rasters.iter().map(Raster::dimensions_mm).collect();
    let report = compose(&dimensions);

    if cli.dry_run {
        for raster in &rasters {
            println!(
                "{} (page {}) {}x{} px",
                raster.path, raster.page_index, raster.width, raster.height
            );
        }
        println!(
            "Sortie prevue : {}",
            resolve_pdf_output(&cli.output)?.display()
        );
        return Ok(());
    }

    let output = resolve_pdf_output(&cli.output)?;
    validate_output(&output, &files, cli.force)?;

    let source_report: Vec<_> = rasters
        .iter()
        .map(|raster| {
            let dims = raster.dimensions_mm();
            json!({
                "path": raster.path,
                "page": raster.page_index,
                "width_mm": dims.width_mm,
                "height_mm": dims.height_mm,
            })
        })
        .collect();

    write_pdf(&output, &rasters, &report)?;

    if let Some(report_path) = cli.report.clone() {
        let report_output = resolve_report_output(&Some(report_path), &files);
        validate_output(&report_output, &files, cli.force)?;
        let envelope = json!({
            "version": 1,
            "sources": source_report,
            "composition": report,
        });
        let text =
            serde_json::to_string_pretty(&envelope).context("serialisation du rapport JSON")?;
        fs::write(&report_output, text)
            .with_context(|| format!("ecriture du rapport {}", report_output.display()))?;
    }

    if !cli.quiet {
        println!(
            "PDF genere : {} ({} sources, {} planches)",
            output.display(),
            rasters.len(),
            report.board_count
        );
    }

    Ok(())
}

fn resolve_pdf_output(output: &Option<PathBuf>) -> Result<PathBuf> {
    let path = output
        .clone()
        .unwrap_or_else(|| PathBuf::from("combinator.pdf"));
    if path.is_dir() {
        return Ok(path.join("combinator.pdf"));
    }

    let has_extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.is_empty())
        .unwrap_or(false);
    if !has_extension {
        return Ok(path.with_extension("pdf"));
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if extension != "pdf" {
        bail!(
            "la sortie principale doit etre un PDF (.pdf), pas .{} ; utilisez --report pour un rapport JSON",
            extension
        );
    }

    Ok(path)
}

fn resolve_report_output(output: &Option<PathBuf>, _files: &[PathBuf]) -> PathBuf {
    output
        .clone()
        .unwrap_or_else(|| PathBuf::from("combinator-report.json"))
}

fn validate_output(output: &Path, sources: &[PathBuf], force: bool) -> Result<()> {
    if sources.iter().any(|source| source == output) {
        bail!(
            "la sortie ne peut pas ecraser une source : {}",
            output.display()
        );
    }
    if output.exists() && !force {
        bail!(
            "la sortie existe deja : {} (utilisez --force)",
            output.display()
        );
    }
    Ok(())
}

fn file_extension_lower(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default()
}

fn raster_from_image(path: &Path) -> Result<Raster> {
    let io_err = |source: std::io::Error| -> anyhow::Error {
        anyhow::anyhow!("lecture de {} : {source}", path.display())
    };

    let bytes = fs::read(path).map_err(io_err)?;
    let mut reader = ImageReader::open(path).map_err(io_err)?;
    reader = reader.with_guessed_format().map_err(|error| {
        anyhow::anyhow!("format image non detecte pour {} : {error}", path.display())
    })?;
    let (width, height) = reader.into_dimensions().map_err(|error| {
        anyhow::anyhow!("dimensions illisibles pour {} : {error}", path.display())
    })?;
    let mut reader = ImageReader::open(path).map_err(io_err)?;
    reader = reader.with_guessed_format().map_err(|error| {
        anyhow::anyhow!("format image non detecte pour {} : {error}", path.display())
    })?;
    let image = reader.decode().map_err(|error| {
        anyhow::anyhow!(
            "decodage image impossible pour {} : {error}",
            path.display()
        )
    })?;
    let rgb_image = image.to_rgb8();
    let (dpi_x, dpi_y) = image_dpi(&bytes);

    Ok(Raster {
        path: path.display().to_string(),
        page_index: 0,
        width,
        height,
        dpi_x,
        dpi_y,
        rgb: rgb_image.into_raw(),
    })
}

fn raster_from_heic(path: &Path) -> Result<Raster> {
    let bytes = fs::read(path).with_context(|| format!("lecture de {}", path.display()))?;
    let info = heic_rs::probe(&bytes)
        .map_err(|error| anyhow::anyhow!("HEIC illisible pour {} : {error}", path.display()))?;

    let layout = match (info.has_alpha, info.bit_depth > 8) {
        (true, true) => heic_rs::PixelLayout::Rgba16,
        (true, false) => heic_rs::PixelLayout::Rgba8,
        (false, true) => heic_rs::PixelLayout::Rgb16,
        (false, false) => heic_rs::PixelLayout::Rgb8,
    };

    let raw = heic_rs::DecodeOptions::default()
        .with_layout(layout)
        .with_transforms(true)
        .with_alpha(info.has_alpha);
    let decoded = heic_rs::decode(&bytes, &raw).map_err(|error| {
        anyhow::anyhow!("decodage HEIC impossible pour {} : {error}", path.display())
    })?;
    let image = build_heic_image(&decoded, layout, info.has_alpha, info.bit_depth > 8)
        .with_context(|| format!("construction image HEIC pour {}", path.display()))?;
    let rgb_image = image.to_rgb8();

    Ok(Raster {
        path: path.display().to_string(),
        page_index: 0,
        width: decoded.width,
        height: decoded.height,
        dpi_x: 300.0,
        dpi_y: 300.0,
        rgb: rgb_image.into_raw(),
    })
}

fn build_heic_image(
    decoded: &heic_rs::Image,
    _layout: heic_rs::PixelLayout,
    has_alpha: bool,
    sixteen_bit: bool,
) -> Result<DynamicImage> {
    let width = decoded.width;
    let height = decoded.height;
    if sixteen_bit {
        let mut samples = Vec::with_capacity(decoded.data.len() / 2);
        for chunk in decoded.data.as_chunks::<2>().0 {
            samples.push(u16::from_le_bytes(*chunk));
        }
        if has_alpha {
            let buffer = ImageBuffer::<image::Rgba<u16>, _>::from_raw(width, height, samples)
                .ok_or_else(|| anyhow::anyhow!("tampon HEIC invalide"))?;
            Ok(DynamicImage::ImageRgba16(buffer))
        } else {
            let buffer = ImageBuffer::<image::Rgb<u16>, _>::from_raw(width, height, samples)
                .ok_or_else(|| anyhow::anyhow!("tampon HEIC invalide"))?;
            Ok(DynamicImage::ImageRgb16(buffer))
        }
    } else if has_alpha {
        let buffer =
            ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, decoded.data.clone())
                .ok_or_else(|| anyhow::anyhow!("tampon HEIC invalide"))?;
        Ok(DynamicImage::ImageRgba8(buffer))
    } else {
        let buffer =
            ImageBuffer::<image::Rgb<u8>, _>::from_raw(width, height, decoded.data.clone())
                .ok_or_else(|| anyhow::anyhow!("tampon HEIC invalide"))?;
        Ok(DynamicImage::ImageRgb8(buffer))
    }
}

fn raster_from_pdf(path: &Path) -> Result<Vec<Raster>> {
    let directory = tempdir().context("creation repertoire temporaire")?;
    let prefix = directory.path().join("page");
    let status = Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("300")
        .arg(path)
        .arg(&prefix)
        .status()
        .with_context(|| format!("lancement de pdftoppm pour {}", path.display()))?;
    if !status.success() {
        bail!("impossible de rasteriser le PDF {}", path.display());
    }

    let mut pngs: Vec<PathBuf> = fs::read_dir(directory.path())?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("png"))
        .collect();
    pngs.sort_by(|left, right| {
        natord::compare(
            left.to_string_lossy().as_ref(),
            right.to_string_lossy().as_ref(),
        )
    });

    if pngs.is_empty() {
        bail!("aucune page rasterisee pour {}", path.display());
    }

    let mut rasters = Vec::new();
    for (page_index, png_path) in pngs.into_iter().enumerate() {
        let raster = raster_from_generated_png(path, page_index, &png_path)?;
        rasters.push(raster);
    }
    Ok(rasters)
}

fn raster_from_generated_png(source: &Path, page_index: usize, png_path: &Path) -> Result<Raster> {
    let bytes = fs::read(png_path).with_context(|| format!("lecture de {}", png_path.display()))?;
    let mut reader = ImageReader::open(png_path)
        .map_err(|error| anyhow::anyhow!("lecture de {} : {error}", png_path.display()))?;
    reader = reader.with_guessed_format().map_err(|error| {
        anyhow::anyhow!(
            "format PNG non detecte pour {} : {error}",
            png_path.display()
        )
    })?;
    let (width, height) = reader.into_dimensions().map_err(|error| {
        anyhow::anyhow!(
            "dimensions illisibles pour {} : {error}",
            png_path.display()
        )
    })?;
    let mut reader = ImageReader::open(png_path)
        .map_err(|error| anyhow::anyhow!("lecture de {} : {error}", png_path.display()))?;
    reader = reader.with_guessed_format().map_err(|error| {
        anyhow::anyhow!(
            "format PNG non detecte pour {} : {error}",
            png_path.display()
        )
    })?;
    let image = reader.decode().map_err(|error| {
        anyhow::anyhow!(
            "decodage PNG impossible pour {} : {error}",
            png_path.display()
        )
    })?;
    let rgb_image = image.to_rgb8();

    let _ = bytes;
    Ok(Raster {
        path: source.display().to_string(),
        page_index,
        width,
        height,
        dpi_x: 300.0,
        dpi_y: 300.0,
        rgb: rgb_image.into_raw(),
    })
}

fn write_pdf(
    output: &Path,
    rasters: &[Raster],
    report: &combinator_core::CompositionReport,
) -> Result<()> {
    let pt_per_mm = 72.0 / 25.4;

    let mut document = Document::with_version("1.7");
    let pages_id = document.new_object_id();

    let mut image_ids = Vec::with_capacity(rasters.len());
    for raster in rasters {
        let image = RgbImage::from_raw(raster.width, raster.height, raster.rgb.clone())
            .ok_or_else(|| anyhow::anyhow!("tampon image invalide"))?;
        let dynamic = DynamicImage::ImageRgb8(image);

        let mut jpeg = Vec::new();
        {
            let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 95);
            encoder
                .encode_image(&dynamic)
                .map_err(|error| anyhow::anyhow!("encodage JPEG : {error}"))?;
        }

        let mut dict = lopdf::Dictionary::new();
        dict.set("Type", Object::Name(b"XObject".to_vec()));
        dict.set("Subtype", Object::Name(b"Image".to_vec()));
        dict.set("Width", Object::Integer(raster.width as i64));
        dict.set("Height", Object::Integer(raster.height as i64));
        dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
        dict.set("BitsPerComponent", Object::Integer(8));
        dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
        dict.set("Length", Object::Integer(jpeg.len() as i64));
        let id = document.add_object(Stream {
            dict,
            content: jpeg,
            allows_compression: false,
            start_position: None,
        });
        image_ids.push(id);
    }

    let mut page_ids = Vec::new();
    for board in &report.boards {
        let mut operations: Vec<Operation> = Vec::new();
        let mut used_images = std::collections::BTreeSet::new();

        for placement in &board.placements {
            let raster_index = placement.source_index;
            let raster = &rasters[raster_index];
            let slot_width = placement.slot_width_mm;
            let slot_height = placement.slot_height_mm;
            let placed_width = placement.placed_width_mm;
            let placed_height = placement.placed_height_mm;

            let center_x_mm = placement.slot_x_mm + (slot_width - placed_width) / 2.0;
            let center_y_mm = placement.slot_y_mm + (slot_height - placed_height) / 2.0;

            let x_pt = center_x_mm * pt_per_mm;
            let y_pt = center_y_mm * pt_per_mm;
            let w_pt = placed_width * pt_per_mm;
            let h_pt = placed_height * pt_per_mm;

            operations.push(Operation::new("q", vec![]));
            if placement.rotation_degrees == 90 {
                operations.push(Operation::new(
                    "cm",
                    vec![
                        0.0.into(),
                        h_pt.into(),
                        (-w_pt).into(),
                        0.0.into(),
                        (x_pt + w_pt).into(),
                        y_pt.into(),
                    ],
                ));
            } else {
                operations.push(Operation::new(
                    "cm",
                    vec![
                        w_pt.into(),
                        0.0.into(),
                        0.0.into(),
                        h_pt.into(),
                        x_pt.into(),
                        y_pt.into(),
                    ],
                ));
            }
            operations.push(Operation::new(
                "Do",
                vec![Object::Name(format!("Im{raster_index}").into_bytes())],
            ));
            operations.push(Operation::new("Q", vec![]));

            used_images.insert(raster_index);
            let _ = raster;
        }

        let content_bytes = {
            let content = Content { operations };
            content
                .encode()
                .map_err(|error| anyhow::anyhow!("encodage du contenu PDF : {error}"))?
        };

        let mut content_dict = lopdf::Dictionary::new();
        content_dict.set("Length", Object::Integer(content_bytes.len() as i64));
        let content_id = document.add_object(Stream {
            dict: content_dict,
            content: content_bytes,
            allows_compression: true,
            start_position: None,
        });

        let mut xobjects = lopdf::Dictionary::new();
        for index in &used_images {
            xobjects.set(
                format!("Im{index}").into_bytes(),
                Object::Reference(image_ids[*index]),
            );
        }
        let mut resources = lopdf::Dictionary::new();
        resources.set("XObject", Object::Dictionary(xobjects));

        let a4_width_pt = combinator_core::A4_WIDTH_MM * pt_per_mm;
        let a4_height_pt = combinator_core::A4_HEIGHT_MM * pt_per_mm;
        let media_box = Object::Array(vec![
            0.0.into(),
            0.0.into(),
            a4_width_pt.into(),
            a4_height_pt.into(),
        ]);

        let mut page_dict = lopdf::Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set("MediaBox", media_box);
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources));

        let page_id = document.add_object(page_dict);
        page_ids.push(page_id);
    }

    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages_dict.set("Count", Object::Integer(page_ids.len() as i64));
    document
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = document.new_object_id();
    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_id));
    document
        .objects
        .insert(catalog_id, Object::Dictionary(catalog));
    document.trailer.set("Root", Object::Reference(catalog_id));

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creation du repertoire {}", parent.display()))?;
    }
    document
        .save(output)
        .with_context(|| format!("ecriture du PDF {}", output.display()))?;
    Ok(())
}

fn image_dpi(bytes: &[u8]) -> (f32, f32) {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        png_dpi(bytes).unwrap_or((300.0, 300.0))
    } else if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        jpeg_dpi(bytes).unwrap_or((300.0, 300.0))
    } else {
        (300.0, 300.0)
    }
}

fn png_dpi(bytes: &[u8]) -> Option<(f32, f32)> {
    let mut i = 8usize;
    while i + 12 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[i..i + 4].try_into().ok()?) as usize;
        if i + 8 + len + 4 > bytes.len() {
            break;
        }
        let typ = &bytes[i + 4..i + 8];
        let data_start = i + 8;
        if typ == b"pHYs" && len >= 9 {
            let px = u32::from_be_bytes(bytes[data_start..data_start + 4].try_into().ok()?);
            let py = u32::from_be_bytes(bytes[data_start + 4..data_start + 8].try_into().ok()?);
            let unit = bytes[data_start + 8];
            if unit == 1 {
                return Some((px as f32 / 39.370_08, py as f32 / 39.370_08));
            }
        }
        i = data_start + len + 4;
    }
    None
}

fn jpeg_dpi(bytes: &[u8]) -> Option<(f32, f32)> {
    let mut i = 2usize;
    let n = bytes.len();
    while i + 1 < n {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        if marker == 0x00 {
            i += 2;
            continue;
        }
        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if (0xD0..=0xD9).contains(&marker) && marker != 0xD8 {
            i += 2;
            continue;
        }
        if i + 4 > n {
            break;
        }
        let seglen = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        if seglen < 2 || i + 2 + seglen > n {
            break;
        }
        if marker == 0xE0 && seglen >= 14 && &bytes[i + 4..i + 8] == b"JFIF\0" {
            let units = bytes[i + 11];
            let x = u16::from_be_bytes([bytes[i + 12], bytes[i + 13]]);
            let y = u16::from_be_bytes([bytes[i + 14], bytes[i + 15]]);
            if units == 1 {
                return Some((x as f32, y as f32));
            }
            if units == 2 {
                return Some((x as f32 * 2.54, y as f32 * 2.54));
            }
        }
        i += 2 + seglen;
    }
    None
}

fn source_error_to_anyhow(error: SourceError) -> anyhow::Error {
    anyhow::anyhow!("{error}")
}

fn expand_directory(path: &Path) -> Result<Vec<PathBuf>, SourceError> {
    combinator_core::expand_directory(path)
}

fn is_supported_path(path: &Path) -> bool {
    combinator_core::is_supported_path(path)
}

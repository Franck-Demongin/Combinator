//! Extraction des dimensions physiques des sources et exploration de repertoire.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use zip::ZipArchive;

use lopdf::Object;

use crate::composition::SourceDimensions;
use crate::error::SourceError;

pub fn supported_extensions() -> &'static [&'static str] {
    &["png", "jpg", "jpeg", "webp", "pdf", "heic", "heif"]
}

pub fn is_supported_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext = ext.to_lowercase();
            supported_extensions().contains(&ext.as_str())
        })
        .unwrap_or(false)
}

pub fn expand_directory(path: &Path) -> Result<Vec<PathBuf>, SourceError> {
    let read_dir = fs::read_dir(path).map_err(|source| SourceError::ReadDirectory {
        path: path.to_path_buf(),
        source,
    })?;

    let mut entries = Vec::new();
    for item in read_dir {
        let item = item.map_err(|source| SourceError::ReadDirectory {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = item.path();
        let file_type = item
            .file_type()
            .map_err(|source| SourceError::ReadDirectory {
                path: path.to_path_buf(),
                source,
            })?;
        if !file_type.is_file() || !is_supported_path(&entry_path) {
            continue;
        }
        entries.push(entry_path);
    }

    entries.sort_by(|a, b| {
        natord::compare(a.to_string_lossy().as_ref(), b.to_string_lossy().as_ref())
    });
    Ok(entries)
}

pub fn dimensions_from_path(path: &Path) -> Result<Vec<SourceDimensions>, SourceError> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" => image_dimensions_mm(path),
        "pdf" => pdf_dimensions_mm(path),
        "heic" | "heif" => heic_dimensions_mm(path),
        _ => Err(SourceError::UnsupportedFormat {
            path: path.to_path_buf(),
            extension,
        }),
    }
}

fn image_dimensions_mm(path: &Path) -> Result<Vec<SourceDimensions>, SourceError> {
    let io_err = |source: std::io::Error| SourceError::Io {
        path: path.to_path_buf(),
        source,
    };

    let mut reader = image::ImageReader::open(path).map_err(io_err)?;
    reader = reader
        .with_guessed_format()
        .map_err(|error| SourceError::Image {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    let (width_px, height_px) = reader
        .into_dimensions()
        .map_err(|error| SourceError::Image {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;

    let bytes = fs::read(path).map_err(io_err)?;
    let (dpi_x, dpi_y) = image_dpi(&bytes);

    let width_mm = width_px as f64 / dpi_x as f64 * 25.4;
    let height_mm = height_px as f64 / dpi_y as f64 * 25.4;
    let dims = SourceDimensions::new(width_mm, height_mm).ok_or_else(|| {
        SourceError::InvalidDimensions {
            path: path.to_path_buf(),
        }
    })?;
    Ok(vec![dims])
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
                let dpi_x = px as f32 / 39.370_08;
                let dpi_y = py as f32 / 39.370_08;
                return Some((dpi_x, dpi_y));
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

fn pdf_dimensions_mm(path: &Path) -> Result<Vec<SourceDimensions>, SourceError> {
    let doc = lopdf::Document::load(path).map_err(|error| SourceError::Pdf {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;

    if doc.is_encrypted() {
        return Err(SourceError::Pdf {
            path: path.to_path_buf(),
            reason: "PDF chiffre non pris en charge".to_owned(),
        });
    }

    let mut pages: Vec<(u32, lopdf::ObjectId)> = doc.get_pages().into_iter().collect();
    pages.sort_by_key(|(number, _)| *number);
    if pages.is_empty() {
        return Err(SourceError::Pdf {
            path: path.to_path_buf(),
            reason: "aucune page".to_owned(),
        });
    }

    let mut out = Vec::with_capacity(pages.len());
    for (_, page_id) in pages {
        let media = inherited_media_box(&doc, page_id).ok_or_else(|| SourceError::Pdf {
            path: path.to_path_buf(),
            reason: "MediaBox introuvable".to_owned(),
        })?;
        let width_pt = (media[2] - media[0]) as f64;
        let height_pt = (media[3] - media[1]) as f64;

        let user_unit = inherited_number(&doc, page_id, b"UserUnit").unwrap_or(1.0) as f64;
        let rotate = inherited_integer(&doc, page_id, b"Rotate").unwrap_or(0);

        let mut width_mm = width_pt * user_unit * 25.4 / 72.0;
        let mut height_mm = height_pt * user_unit * 25.4 / 72.0;
        if matches!(rotate.rem_euclid(360), 90 | 270) {
            std::mem::swap(&mut width_mm, &mut height_mm);
        }

        let dims = SourceDimensions::new(width_mm, height_mm).ok_or_else(|| {
            SourceError::InvalidDimensions {
                path: path.to_path_buf(),
            }
        })?;
        out.push(dims);
    }
    Ok(out)
}

fn inherited_media_box(doc: &lopdf::Document, start: lopdf::ObjectId) -> Option<[f32; 4]> {
    let mut current = start;
    let mut visited = HashSet::new();
    while visited.insert(current) {
        let dict = doc.get_object(current).ok()?.as_dict().ok()?;
        if let Ok(array) = dict.get(b"MediaBox").and_then(|value| value.as_array())
            && array.len() >= 4
        {
            let mut values = [0.0f32; 4];
            let mut ok = true;
            for (index, item) in array.iter().take(4).enumerate() {
                values[index] = as_f32(item).unwrap_or_else(|| {
                    ok = false;
                    0.0
                });
                if !ok {
                    break;
                }
            }
            if ok {
                return Some(values);
            }
        }
        match dict.get(b"Parent") {
            Ok(Object::Reference(next)) => current = *next,
            _ => break,
        }
    }
    None
}

fn inherited_number(doc: &lopdf::Document, start: lopdf::ObjectId, key: &[u8]) -> Option<f32> {
    let mut current = start;
    let mut visited = HashSet::new();
    while visited.insert(current) {
        let dict = doc.get_object(current).ok()?.as_dict().ok()?;
        if let Ok(value) = dict.get(key) {
            if let Ok(number) = value.as_float() {
                return Some(number);
            }
            if let Ok(number) = value.as_i64() {
                return Some(number as f32);
            }
        }
        match dict.get(b"Parent") {
            Ok(Object::Reference(next)) => current = *next,
            _ => break,
        }
    }
    None
}

fn inherited_integer(doc: &lopdf::Document, start: lopdf::ObjectId, key: &[u8]) -> Option<i64> {
    let mut current = start;
    let mut visited = HashSet::new();
    while visited.insert(current) {
        let dict = doc.get_object(current).ok()?.as_dict().ok()?;
        if let Ok(value) = dict.get(key) {
            if let Ok(number) = value.as_i64() {
                return Some(number);
            }
            if let Ok(number) = value.as_float() {
                return Some(number as i64);
            }
        }
        match dict.get(b"Parent") {
            Ok(Object::Reference(next)) => current = *next,
            _ => break,
        }
    }
    None
}

fn as_f32(value: &Object) -> Option<f32> {
    if let Ok(number) = value.as_float() {
        Some(number)
    } else if let Ok(number) = value.as_i64() {
        Some(number as f32)
    } else {
        None
    }
}

static ZIP_EXTRACTION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn expand_zip(path: &Path) -> Result<Vec<PathBuf>, SourceError> {
    let file = fs::File::open(path).map_err(|source| SourceError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut archive = ZipArchive::new(file).map_err(|error| SourceError::Zip {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;

    let extraction_root = std::env::temp_dir().join(format!(
        "combinator-zip-{}-{}",
        std::process::id(),
        ZIP_EXTRACTION_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&extraction_root).map_err(|source| SourceError::Io {
        path: extraction_root.clone(),
        source,
    })?;

    let mut extracted = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| SourceError::Zip {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;

        // Certaines archives stockent des noms UTF-8 sans positionner le bit
        // UTF-8 de l'en-tête ZIP. On préfère alors décoder les octets bruts
        // comme UTF-8 pour conserver les accents.
        let entry_name = std::str::from_utf8(entry.name_raw())
            .map(str::to_owned)
            .unwrap_or_else(|_| entry.name().to_owned());

        let Some(target) = zip_entry_target(&extraction_root, &entry_name) else {
            continue;
        };

        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|source| SourceError::Io {
                path: target.clone(),
                source,
            })?;
            continue;
        }

        if !is_supported_path(Path::new(&entry_name)) {
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|source| SourceError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let mut output = fs::File::create(&target).map_err(|source| SourceError::Io {
            path: target.clone(),
            source,
        })?;
        std::io::copy(&mut entry, &mut output).map_err(|error| SourceError::Zip {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;

        extracted.push(target);
    }

    extracted.sort_by(|a, b| {
        natord::compare(a.to_string_lossy().as_ref(), b.to_string_lossy().as_ref())
    });
    Ok(extracted)
}

fn zip_entry_target(root: &Path, name: &str) -> Option<PathBuf> {
    use std::path::Component;

    let mut target = root.to_path_buf();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => target.push(part),
            _ => return None,
        }
    }
    Some(target)
}
fn heic_dimensions_mm(path: &Path) -> Result<Vec<SourceDimensions>, SourceError> {
    let bytes = fs::read(path).map_err(|source| SourceError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let info = heic_rs::probe(&bytes).map_err(|error| SourceError::Heic {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;

    // Les images HEIC n'exposent pas toujours un DPI exploitable.
    // On reprend la meme convention que les images (300 dpi par defaut).
    let width_mm = info.width as f64 / 300.0 * 25.4;
    let height_mm = info.height as f64 / 300.0 * 25.4;
    let dims = SourceDimensions::new(width_mm, height_mm).ok_or_else(|| {
        SourceError::InvalidDimensions {
            path: path.to_path_buf(),
        }
    })?;
    Ok(vec![dims])
}

//! Erreurs de lecture et d'extraction de dimensions des sources.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("lecture de fichier impossible ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("format non supporté pour {path}: {extension}")]
    UnsupportedFormat { path: PathBuf, extension: String },
    #[error("image illisible ({path}): {reason}")]
    Image { path: PathBuf, reason: String },
    #[error("PDF illisible ({path}): {reason}")]
    Pdf { path: PathBuf, reason: String },
    #[error("HEIC illisible ({path}): {reason}")]
    Heic { path: PathBuf, reason: String },
    #[error("dimensions invalides pour {path}")]
    InvalidDimensions { path: PathBuf },
    #[error("archive ZIP illisible ({path}): {reason}")]
    Zip { path: PathBuf, reason: String },
    #[error("répertoire illisible ({path}): {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

use anyhow::{Context, Result, bail};
use clap::Parser;
use combinator_core::{
    SourceDimensions, SourceError, compose, dimensions_from_path, expand_directory,
    is_supported_path,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

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
    #[arg(value_name = "INPUT", trailing_var_arg = true)]
    inputs: Vec<PathBuf>,

    /// Chemin du rapport JSON genere.
    #[arg(short = 'o', long = "output", value_name = "PATH")]
    output: Option<PathBuf>,

    /// Ecrase un rapport existant.
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// Affiche le plan sans ecrire le rapport.
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Sortie silencieuse en cas de succes.
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
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

    if cli.dry_run {
        for file in &files {
            match dimensions_from_path(file) {
                Ok(dimensions) => {
                    println!("{} ({} source(s))", file.display(), dimensions.len());
                }
                Err(error) => {
                    println!("{} (erreur: {error})", file.display());
                }
            }
        }
        let output = cli
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from("combinator-report.json"));
        println!(
            "Sortie prevue : {}",
            resolve_output(&Some(output), &files)?.display()
        );
        return Ok(());
    }

    let output = resolve_output(&cli.output, &files)?;
    validate_output(&output, &files, cli.force)?;

    let mut dimensions_list: Vec<SourceDimensions> = Vec::new();
    let mut source_report = Vec::new();
    let mut errors = Vec::new();

    for file in &files {
        match dimensions_from_path(file) {
            Ok(dimensions) => {
                for (page_index, dimension) in dimensions.into_iter().enumerate() {
                    dimensions_list.push(dimension);
                    source_report.push(json!({
                        "path": file.display().to_string(),
                        "page": page_index,
                        "width_mm": dimension.width_mm,
                        "height_mm": dimension.height_mm,
                    }));
                }
            }
            Err(error) => {
                errors.push(json!({
                    "path": file.display().to_string(),
                    "error": error.to_string(),
                }));
            }
        }
    }

    if dimensions_list.is_empty() {
        bail!("aucune source valide n'a pu etre lue");
    }

    let report = compose(&dimensions_list);

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creation du repertoire {}", parent.display()))?;
    }
    if output.exists() && cli.force {
        fs::remove_file(&output).with_context(|| format!("suppression de {}", output.display()))?;
    }

    let envelope = json!({
        "version": 1,
        "sources": source_report,
        "errors": errors,
        "composition": report,
    });

    let text = serde_json::to_string_pretty(&envelope).context("serialisation du rapport JSON")?;
    fs::write(&output, text)
        .with_context(|| format!("ecriture du rapport {}", output.display()))?;

    if !cli.quiet {
        println!(
            "Rapport genere : {} ({} sources, {} planches)",
            output.display(),
            dimensions_list.len(),
            report.board_count
        );
    }

    Ok(())
}

fn resolve_output(output: &Option<PathBuf>, files: &[PathBuf]) -> Result<PathBuf> {
    let path = output
        .clone()
        .unwrap_or_else(|| PathBuf::from("combinator-report.json"));

    let resolved = if path.is_dir() {
        path.join("combinator-report.json")
    } else {
        let has_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| !ext.is_empty())
            .unwrap_or(false);
        if has_extension {
            path
        } else {
            path.with_extension("json")
        }
    };

    let _ = files;
    Ok(resolved)
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

fn source_error_to_anyhow(error: SourceError) -> anyhow::Error {
    anyhow::anyhow!("{error}")
}

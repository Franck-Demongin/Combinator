use assert_cmd::Command;
use image::{ImageBuffer, Rgb};
use lopdf::{Object, dictionary};
use std::fs;
use tempfile::TempDir;

fn combinator() -> Command {
    Command::cargo_bin("combinator").expect("binary")
}

fn make_png(path: &std::path::Path, width: u32, height: u32) {
    let image = ImageBuffer::<Rgb<u8>, _>::from_fn(width, height, |_, _| Rgb([255, 0, 0]));
    image.save(path).expect("png");
}

fn make_pdf(path: &std::path::Path, width: i64, height: i64) {
    let mut document = lopdf::Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let page_id = document.new_object_id();
    let media_box = Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(width),
        Object::Integer(height),
    ]);
    let page = dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => media_box,
    };
    document.set_object(page_id, Object::Dictionary(page));
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => Object::Array(vec![Object::Reference(page_id)]),
        "Count" => Object::Integer(1),
    };
    document.set_object(pages_id, Object::Dictionary(pages));
    let catalog_id = document.new_object_id();
    let catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    };
    document.set_object(catalog_id, Object::Dictionary(catalog));
    document.trailer.set("Root", Object::Reference(catalog_id));
    document.save(path).expect("pdf");
}

#[test]
fn help_succeeds() {
    combinator()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Composition de sources"));
}

#[test]
fn directory_is_single_level_and_writes_json() {
    let temp = TempDir::new().expect("temp");
    make_png(&temp.path().join("b.png"), 100, 50);
    make_png(&temp.path().join("a.png"), 10, 20);
    make_pdf(&temp.path().join("page.pdf"), 200, 100);
    fs::create_dir(temp.path().join("nested")).expect("nested");
    make_png(&temp.path().join("nested").join("ignored.png"), 1, 1);
    fs::write(temp.path().join("ignore.txt"), "x").expect("txt");

    let output = temp.path().join("report.json");
    combinator()
        .arg("--dir")
        .arg(temp.path())
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&output).expect("read")).expect("json");
    let sources = value["sources"].as_array().expect("sources");
    assert_eq!(sources.len(), 3, "nested directory and txt must be ignored");
    let first_paths: Vec<_> = sources
        .iter()
        .map(|item| item["path"].as_str().unwrap())
        .collect();
    assert!(first_paths.iter().all(|path| !path.contains("nested")));
}

#[test]
fn existing_output_requires_force() {
    let temp = TempDir::new().expect("temp");
    make_png(&temp.path().join("a.png"), 10, 10);
    let output = temp.path().join("report.json");
    fs::write(&output, "existing").expect("existing");

    combinator()
        .arg("--dir")
        .arg(temp.path())
        .arg("-o")
        .arg(&output)
        .assert()
        .failure();

    assert_eq!(fs::read_to_string(&output).expect("read"), "existing");

    combinator()
        .arg("--dir")
        .arg(temp.path())
        .arg("-o")
        .arg(&output)
        .arg("--force")
        .assert()
        .success();
}

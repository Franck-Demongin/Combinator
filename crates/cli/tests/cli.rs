use assert_cmd::Command;
use image::{ImageBuffer, Rgb};
use lopdf::Document;
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
    make_pdf_pages(path, width, height, 1);
}

fn make_webp(path: &std::path::Path, width: u32, height: u32) {
    let image = image::ImageBuffer::<image::Rgb<u8>, _>::from_fn(width, height, |_, _| {
        image::Rgb([10, 20, 30])
    });
    image.save(path).expect("webp");
}

fn make_pdf_pages(path: &std::path::Path, width: i64, height: i64, page_count: usize) {
    use lopdf::{Object, dictionary};
    let mut document = lopdf::Document::with_version("1.5");
    let pages_id = document.new_object_id();

    let media_box = Object::Array(vec![
        Object::Integer(0),
        Object::Integer(0),
        Object::Integer(width),
        Object::Integer(height),
    ]);

    let mut page_ids = Vec::new();
    for _ in 0..page_count.max(1) {
        let page_id = document.new_object_id();
        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => media_box.clone(),
        };
        document.set_object(page_id, Object::Dictionary(page));
        page_ids.push(page_id);
    }

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => Object::Array(page_ids.iter().map(|id| Object::Reference(*id)).collect()),
        "Count" => Object::Integer(page_ids.len() as i64),
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
fn directory_is_single_level_and_writes_pdf() {
    let temp = TempDir::new().expect("temp");
    make_png(&temp.path().join("b.png"), 100, 50);
    make_png(&temp.path().join("a.png"), 10, 20);
    make_pdf(&temp.path().join("page.pdf"), 200, 100);
    fs::create_dir(temp.path().join("nested")).expect("nested");
    make_png(&temp.path().join("nested").join("ignored.png"), 1, 1);
    fs::write(temp.path().join("ignore.txt"), "x").expect("txt");

    let output = temp
        .path()
        .parent()
        .unwrap()
        .join("combinator-test-report.pdf");
    let _ = fs::remove_file(&output);
    combinator()
        .arg("--dir")
        .arg(temp.path())
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let bytes = fs::read(&output).expect("read pdf");
    assert!(bytes.starts_with(b"%PDF"), "output should be a PDF");
    let document = Document::load(&output).expect("valid pdf");
    assert_eq!(
        document.get_pages().len(),
        2,
        "3 sources should produce 2 A4 boards"
    );
}

#[test]
fn multi_page_pdf_is_treated_as_independent_pages() {
    let temp = TempDir::new().expect("temp");
    make_pdf_pages(&temp.path().join("pages.pdf"), 200, 100, 4);

    let output = temp
        .path()
        .parent()
        .unwrap()
        .join("combinator-multi-page-report.pdf");
    let _ = fs::remove_file(&output);
    combinator()
        .arg("--source")
        .arg(temp.path().join("pages.pdf"))
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let bytes = fs::read(&output).expect("read pdf");
    assert!(bytes.starts_with(b"%PDF"), "output should be a PDF");
    let document = Document::load(&output).expect("valid pdf");
    assert_eq!(
        document.get_pages().len(),
        2,
        "4 PDF pages should produce 2 A4 boards"
    );
}

#[test]
fn multi_page_pdf_report_lists_every_page() {
    let temp = TempDir::new().expect("temp");
    make_pdf_pages(&temp.path().join("pages.pdf"), 200, 100, 4);

    let output = temp
        .path()
        .parent()
        .unwrap()
        .join("combinator-multi-page-report-2.pdf");
    let report = temp
        .path()
        .parent()
        .unwrap()
        .join("combinator-multi-page-report-2.json");
    let _ = fs::remove_file(&output);
    let _ = fs::remove_file(&report);
    combinator()
        .arg("--source")
        .arg(temp.path().join("pages.pdf"))
        .arg("-o")
        .arg(&output)
        .arg("--report")
        .arg(&report)
        .assert()
        .success();

    let report_text = fs::read_to_string(&report).expect("read report");
    for page_index in 0..4 {
        assert!(
            report_text.contains(&format!(r#""page": {page_index}"#)),
            "report should mention page {page_index}"
        );
    }
}

#[test]
fn webp_source_is_supported() {
    let temp = TempDir::new().expect("temp");
    make_webp(&temp.path().join("a.webp"), 20, 10);

    let output = temp.path().parent().unwrap().join("combinator-webp.pdf");
    let _ = fs::remove_file(&output);
    combinator()
        .arg("--source")
        .arg(temp.path().join("a.webp"))
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let bytes = fs::read(&output).expect("read pdf");
    assert!(bytes.starts_with(b"%PDF"));
    let document = Document::load(&output).expect("valid pdf");
    assert_eq!(document.get_pages().len(), 1);
}

#[test]
fn existing_output_requires_force() {
    let temp = TempDir::new().expect("temp");
    make_png(&temp.path().join("a.png"), 10, 10);
    let output = temp
        .path()
        .parent()
        .unwrap()
        .join("combinator-force-report.pdf");
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

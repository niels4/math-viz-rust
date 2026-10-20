//! The render scene judged against the twin.
//!
//! SDL needs the main thread, which `cargo test` does not run tests on, so
//! the scene renders in `examples/render_scene.rs` (same numbers, same kb
//! call sequence) and this test drives that binary and judges its PNG:
//! identical to the twin's at threshold 0 per channel, with the differing
//! pixels mapped on failure; the kerned 48px advances within 0.01px of the
//! twin's.

const PX_W: u32 = 800;
const PX_H: u32 = 400;

// The twin's numbers for this scene, font and sizes (tools/twin output).
const TWIN_ADVANCE_48: f64 = 556.608;
const TWIN_ADVANCE_48_BOLD: f64 = 562.320;

fn shot_dir() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.local/shots/t0c-b");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn example_binary() -> std::path::PathBuf {
    // target/debug/deps/render-<hash> -> target/debug/examples/render_scene
    let exe = std::env::current_exe().unwrap();
    exe.parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("examples")
        .join(format!("render_scene{}", std::env::consts::EXE_SUFFIX))
}

fn advance_from(stdout: &str, key: &str) -> f64 {
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest.trim().parse().expect("advance number");
        }
    }
    panic!("{key} missing in render output:\n{stdout}");
}

#[test]
fn render_matches_twin() {
    let dir = shot_dir();
    let rust_path = dir.join("rust-render.png");
    let out = std::process::Command::new(example_binary())
        .arg("--out")
        .arg(&rust_path)
        .output()
        .expect("render_scene binary runs");
    assert!(
        out.status.success(),
        "render_scene failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    print!("{stdout}");
    let advance48 = advance_from(&stdout, "rust: advance48=");
    assert!(
        (advance48 - TWIN_ADVANCE_48).abs() < 0.01,
        "kerned 48px advance {advance48} vs twin {TWIN_ADVANCE_48}"
    );
    let advance_bold = advance_from(&stdout, "rust: advance48bold=");
    assert!(
        (advance_bold - TWIN_ADVANCE_48_BOLD).abs() < 0.01,
        "kerned bold advance {advance_bold} vs twin {TWIN_ADVANCE_48_BOLD}"
    );

    let twin_path = dir.join("twin.png");
    assert!(
        twin_path.exists(),
        "run tools/twin/run first: {} missing",
        twin_path.display()
    );
    let twin = read_png(&twin_path);
    let rust = read_png(&rust_path);
    assert_eq!(rust.len(), twin.len(), "twin PNG size");
    let mut diff = 0u32;
    let mut diff_map = vec![0u8; rust.len()];
    for (i, (a, b)) in rust.iter().zip(twin.iter()).enumerate() {
        if a != b {
            diff += 1;
            let px = i / 4;
            diff_map[px * 4] = 255;
            diff_map[px * 4 + 3] = 255;
        } else {
            diff_map[i] = *a;
        }
    }
    println!("rust: differing pixels vs twin at threshold 0: {diff}");
    if diff != 0 {
        let diff_path = dir.join("rust-diff.png");
        let file = std::fs::File::create(&diff_path).unwrap();
        let mut enc = png::Encoder::new(file, PX_W, PX_H);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .unwrap()
            .write_image_data(&diff_map)
            .unwrap();
        panic!(
            "{diff} pixels differ from the twin (map: {})",
            diff_path.display()
        );
    }
}

fn read_png(path: &std::path::Path) -> Vec<u8> {
    let file = std::fs::File::open(path).unwrap();
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0u8; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.width, PX_W);
    assert_eq!(info.height, PX_H);
    assert!(matches!(info.color_type, png::ColorType::Rgba));
    buf
}

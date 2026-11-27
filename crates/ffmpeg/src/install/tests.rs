use super::*;
use std::net::TcpListener;

fn temp(name: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("ffmpeg-inst-{name}"))
}

fn members() -> Vec<(String, Vec<u8>)> {
    let top = "ffmpeg-test-build";
    vec![
        (
            format!("{top}/bin/{}", Tool::Ffmpeg.file_name()),
            b"fake ffmpeg".to_vec(),
        ),
        (
            format!("{top}/bin/{}", Tool::Ffprobe.file_name()),
            b"fake ffprobe".to_vec(),
        ),
        (format!("{top}/bin/ffplay"), b"not kept".to_vec()),
        (format!("{top}/{LICENSE}"), b"LGPL text".to_vec()),
        (format!("{top}/doc/other.txt"), b"not kept".to_vec()),
    ]
}

fn tar_xz() -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (name, data) in members() {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append_data(&mut header, name, data.as_slice()).unwrap();
    }
    let raw = tar.into_inner().unwrap();
    let mut xz = lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default()).unwrap();
    xz.write_all(&raw).unwrap();
    xz.finish().unwrap()
}

fn zipped() -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in members() {
        zip.start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn pin_for(bytes: &[u8], archive: Archive) -> Pin {
    let sha256 = hex(&Sha256::digest(bytes));
    Pin {
        version: "test",
        url: "unused",
        sha256: Box::leak(sha256.into_boxed_str()),
        size: bytes.len() as u64,
        archive,
        license: "LGPL-3.0-or-later",
    }
}

/// Serves `bytes` to the next `requests` connections; returns the URL.
fn serve(bytes: Vec<u8>, requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/ffmpeg", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for _ in 0..requests {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&bytes);
        }
    });
    url
}

fn assert_installed(dir: &Path) {
    assert_eq!(
        std::fs::read(dir.join(Tool::Ffmpeg.file_name())).unwrap(),
        b"fake ffmpeg"
    );
    assert_eq!(
        std::fs::read(dir.join(Tool::Ffprobe.file_name())).unwrap(),
        b"fake ffprobe"
    );
    assert_eq!(std::fs::read(dir.join(LICENSE)).unwrap(), b"LGPL text");
    assert!(!dir.join("ffplay").exists());
    assert_eq!(std::fs::read_dir(dir).unwrap().count(), 3);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.join("ffmpeg"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111, "the executable bit");
    }
}

fn leftovers(root: &Path) -> Vec<String> {
    std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn a_matching_tar_xz_installs_the_two_tools_and_the_license() {
    let bytes = tar_xz();
    let pin = pin_for(&bytes, Archive::TarXz);
    let root = temp("tarxz");
    let progress = Progress::default();
    let dir = install_from(&serve(bytes.clone(), 1), &pin, &root, &progress).unwrap();
    assert_eq!(dir, root.join("ffmpeg-test"));
    assert_installed(&dir);
    assert_eq!(progress.done.load(Ordering::Relaxed), bytes.len() as u64);
    assert_eq!(leftovers(&root), ["ffmpeg-test"]);
}

#[test]
fn a_matching_zip_installs_the_same() {
    let bytes = zipped();
    let pin = pin_for(&bytes, Archive::Zip);
    let root = temp("zip");
    let dir = install_from(&serve(bytes, 1), &pin, &root, &Progress::default()).unwrap();
    assert_installed(&dir);
}

#[test]
fn a_hash_mismatch_installs_nothing_and_leaves_nothing_behind() {
    let bytes = tar_xz();
    let mut pin = pin_for(&bytes, Archive::TarXz);
    pin.sha256 = &"0".repeat(64).leak()[..];
    let root = temp("mismatch");
    let err = install_from(&serve(bytes, 1), &pin, &root, &Progress::default()).unwrap_err();
    assert!(format!("{err:#}").contains("SHA-256"), "{err:#}");
    assert!(leftovers(&root).is_empty(), "{:?}", leftovers(&root));
}

#[test]
fn a_cancel_installs_nothing_and_leaves_nothing_behind() {
    let bytes = tar_xz();
    let pin = pin_for(&bytes, Archive::TarXz);
    let root = temp("cancel");
    let progress = Progress::default();
    progress.cancel.store(true, Ordering::Relaxed);
    let err = install_from(&serve(bytes, 1), &pin, &root, &progress).unwrap_err();
    assert!(err.downcast_ref::<Cancelled>().is_some(), "{err:#}");
    assert!(leftovers(&root).is_empty());
}

#[test]
fn an_archive_without_the_tools_installs_nothing() {
    let mut tar = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(1);
    header.set_cksum();
    tar.append_data(&mut header, "top/README", &b"x"[..])
        .unwrap();
    let raw = tar.into_inner().unwrap();
    let mut xz = lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default()).unwrap();
    xz.write_all(&raw).unwrap();
    let bytes = xz.finish().unwrap();
    let pin = pin_for(&bytes, Archive::TarXz);
    let root = temp("empty");
    let err = install_from(&serve(bytes, 1), &pin, &root, &Progress::default()).unwrap_err();
    assert!(format!("{err:#}").contains("has no"), "{err:#}");
    assert!(leftovers(&root).is_empty());
}

#[test]
fn an_installed_build_is_not_downloaded_again() {
    let bytes = tar_xz();
    let pin = pin_for(&bytes, Archive::TarXz);
    let root = temp("again");
    install_from(&serve(bytes, 1), &pin, &root, &Progress::default()).unwrap();
    // Nothing is listening on this one: a second download would fail.
    install_from("http://127.0.0.1:1/none", &pin, &root, &Progress::default()).unwrap();
}

#[test]
fn an_unreachable_server_is_an_error_not_a_hang() {
    let bytes = tar_xz();
    let pin = pin_for(&bytes, Archive::TarXz);
    let root = temp("offline");
    install_from("http://127.0.0.1:1/none", &pin, &root, &Progress::default()).unwrap_err();
    assert!(leftovers(&root).is_empty());
}

/// The real pinned archive, served from a local copy: proves the xz and tar
/// readers take the publisher's bytes. `OAG_FFMPEG_ARCHIVE` names the file.
#[test]
#[ignore = "needs the real archive in OAG_FFMPEG_ARCHIVE; run by hand"]
fn the_real_pinned_archive_unpacks() {
    let Some(path) = std::env::var_os("OAG_FFMPEG_ARCHIVE") else {
        return;
    };
    let pin = *crate::Pin::current().expect("a pin for this OS");
    let bytes = std::fs::read(path).unwrap();
    let root = temp("real");
    let dir = install_from(&serve(bytes, 1), &pin, &root, &Progress::default()).unwrap();
    for name in [Tool::Ffmpeg.file_name(), Tool::Ffprobe.file_name(), LICENSE] {
        assert!(
            std::fs::metadata(dir.join(name)).unwrap().len() > 1000,
            "{name}"
        );
    }
}

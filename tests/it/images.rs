use std::{
    fs,
    io::Cursor,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

use image::{DynamicImage, GrayImage, ImageFormat, Luma};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    env::{TOKEN, TestEnv},
    server::{FakeServer, Reply, Request},
};

/// A synthetic image group (film), with the zeros the storage requires.
const FILM: &str = "009990001";

/// A synthetic page as a JPEG: gray, darker on the left than on the right.
/// Pages of different widths are different files.
pub(crate) fn page(width: u32, height: u32) -> Vec<u8> {
    let level = |x: u32, _: u32| Luma([u8::try_from(x * 255 / width).unwrap()]);
    encoded(GrayImage::from_fn(width, height, level), ImageFormat::Jpeg)
}

pub(crate) fn encoded(image: GrayImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(image)
        .write_to(&mut bytes, format)
        .unwrap();
    bytes.into_inner()
}

/// The page the fake storage holds for one image of the film.
fn page_of(number: u32) -> Vec<u8> {
    page(40 + number, 30)
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// What a request to the fake image storage asks for.
enum Asked {
    /// How many images the film has.
    Total,
    /// The full image, on FamilySearch: answered with a redirect.
    Full(u32),
    /// The thumbnail of an image.
    Thumbnail(u32),
    /// The bytes of a full image, at the signed address of the storage.
    Signed(u32),
}

fn asked(request: &Request) -> Asked {
    let path = request.path.as_str();
    if path == format!("/dgs:{FILM}") {
        return Asked::Total;
    }
    if let Some(number) = path.strip_prefix("/assinado/") {
        return Asked::Signed(number.split('?').next().unwrap().parse().unwrap());
    }
    let (number, file) = path
        .strip_prefix(&format!("/dgs:{FILM}_"))
        .and_then(|rest| rest.split_once('/'))
        .unwrap_or_else(|| panic!("unexpected route: {path}"));
    let number = number.parse().unwrap();
    match file {
        "dist.jpg" => Asked::Full(number),
        "thumb_p200.jpg" => Asked::Thumbnail(number),
        other => panic!("unexpected file: {other}"),
    }
}

/// The answer of a film of `total` images that the account may see whole.
fn open_film(total: u32, request: &Request) -> Reply {
    match asked(request) {
        Asked::Total => Reply::json(200, json!({"childCount": total}).to_string()),
        // The signature in the address is the credential of the second hop.
        Asked::Full(number) => Reply::empty(302).with_header(
            "Location",
            &format!(
                "http://{}/assinado/{number}?X-Amz-Signature=sintetica",
                request.header("Host").unwrap()
            ),
        ),
        Asked::Signed(number) => Reply::bytes(200, "image/jpeg", page_of(number)),
        Asked::Thumbnail(number) => Reply::bytes(200, "image/jpeg", page(20 + number, 15)),
    }
}

fn manifest(folder: &Path) -> Vec<Value> {
    fs::read_to_string(folder.join("manifesto.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// How many times the full image `number` was asked of FamilySearch.
fn asked_full(fs: &FakeServer, number: u32) -> usize {
    fs.requests()
        .iter()
        .filter(|request| matches!(asked(request), Asked::Full(asked) if asked == number))
        .count()
}

#[test]
fn a_film_downloads_at_full_resolution_with_stable_names_and_a_manifest() {
    let fs = FakeServer::start(|request| open_film(3, request));
    let env = TestEnv::logged_in(&fs);

    // The catalog drops the zeros of the group; the storage needs them.
    let downloaded = env.run(&["imagens", "baixar", "9990001"]);

    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(downloaded.stdout, format!("{FILM}\t3\t0\t0\timagens\n"));
    let folder = env.work_dir().join("imagens");
    let mut lines = vec![json!({"situacao": "listado", "grupo": FILM, "total": 3})];
    for number in 1..=3 {
        let name = format!("{FILM}_0000{number}.jpg");
        let page = page_of(number);
        assert_eq!(fs::read(folder.join(&name)).unwrap(), page, "{name}");
        lines.push(json!({
            "situacao": "baixada", "grupo": FILM, "numero": number,
            "id": format!("dgs:{FILM}_0000{number}"), "arquivo": name,
            "sha256": sha256(&page), "bytes": page.len(), "largura": 40 + number, "altura": 30,
        }));
    }
    assert_eq!(manifest(&folder), lines);
    // Nothing unfinished is left in the folder: three images, the manifest
    // and its lock.
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 5);
    for request in fs.requests() {
        let agent = request.header("User-Agent").unwrap();
        assert!(agent.starts_with("garimpo/"), "{agent}");
        let session = request.header("Authorization");
        match asked(&request) {
            // The session goes to FamilySearch and never to the signed address.
            Asked::Signed(_) => assert_eq!(session, None),
            Asked::Total | Asked::Full(_) | Asked::Thumbnail(_) => {
                assert_eq!(session, Some(format!("Bearer {TOKEN}").as_str()));
            }
        }
    }
}

#[test]
fn a_429_in_the_middle_stops_the_download_and_the_same_command_resumes_without_loss() {
    let limited = Arc::new(AtomicBool::new(true));
    let fs = FakeServer::start({
        let limited = Arc::clone(&limited);
        move |request| match asked(request) {
            Asked::Full(3) if limited.load(Ordering::Relaxed) => {
                Reply::empty(429).with_header("Retry-After", "0")
            }
            _ => open_film(5, request),
        }
    });
    let env = TestEnv::new(&fs).with_var("GARIMPO_PAUSA_429_S", "0");
    env.deliver_session();
    let download = ["imagens", "baixar", FILM, "--paralelo", "3"];

    let blocked = env.run(&download);
    // With the block, no worker asked for the images after it.
    assert_eq!([4, 5].map(|number| asked_full(&fs, number)), [0, 0]);
    limited.store(false, Ordering::Relaxed);
    let resumed = env.run(&download);

    assert_eq!(blocked.code, 4, "{}", blocked.stderr);
    assert!(
        blocked.stderr.contains("repita o mesmo comando"),
        "{}",
        blocked.stderr
    );
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    // The two that came before the block were kept, not downloaded again.
    assert_eq!(resumed.stdout, format!("{FILM}\t3\t2\t0\timagens\n"));
    let folder = env.work_dir().join("imagens");
    for number in 1..=5 {
        let file = folder.join(format!("{FILM}_0000{number}.jpg"));
        assert_eq!(fs::read(file).unwrap(), page_of(number), "image {number}");
    }
    assert_eq!(manifest(&folder).len(), 6);
    assert_eq!(asked_full(&fs, 1), 1);
    assert_eq!(asked_full(&fs, 2), 1);
    // Three attempts while limited, and one more once the limit lifted.
    assert_eq!(asked_full(&fs, 3), 4);
}

#[test]
fn a_resumed_download_checks_each_file_against_the_hash_in_the_manifest() {
    let fs = FakeServer::start(|request| open_film(3, request));
    let env = TestEnv::logged_in(&fs);
    let download = ["imagens", "baixar", FILM];
    assert_eq!(env.run(&download).code, 0);
    let folder = env.work_dir().join("imagens");
    // One file is damaged and another one is gone.
    fs::write(folder.join(format!("{FILM}_00002.jpg")), b"damaged").unwrap();
    fs::remove_file(folder.join(format!("{FILM}_00003.jpg"))).unwrap();

    let resumed = env.run(&download);

    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    assert_eq!(resumed.stdout, format!("{FILM}\t2\t1\t0\timagens\n"));
    for number in 2..=3 {
        let file = folder.join(format!("{FILM}_0000{number}.jpg"));
        assert_eq!(fs::read(file).unwrap(), page_of(number), "image {number}");
    }
    assert_eq!(
        [1, 2, 3].map(|number| asked_full(&fs, number)),
        [1, 2, 2],
        "only the two that did not check were asked again"
    );
    assert_eq!(manifest(&folder).len(), 4);
}

#[test]
fn a_restricted_image_is_left_pending_access_and_is_not_asked_again() {
    let fs = FakeServer::start(|request| match asked(request) {
        Asked::Full(2) => Reply::json(
            403,
            json!({"message": "Access to artifact denied"}).to_string(),
        ),
        _ => open_film(3, request),
    });
    let env = TestEnv::logged_in(&fs);
    let download = ["imagens", "baixar", FILM, "--paralelo", "1"];

    let downloaded = env.run(&download);
    let again = env.run(&download);
    let listed = env.run(&["imagens", "listar", FILM]);

    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(downloaded.stdout, format!("{FILM}\t2\t0\t1\timagens\n"));
    assert!(
        downloaded
            .stderr
            .contains("imagem 2 pendente de acesso: acesso restrito"),
        "{}",
        downloaded.stderr
    );
    assert_eq!(
        manifest(&env.work_dir().join("imagens"))[2],
        json!({
            "situacao": "pendente-de-acesso", "grupo": FILM, "numero": 2,
            "id": format!("dgs:{FILM}_00002"),
            "motivo": format!(
                "acesso restrito: /dgs:{FILM}_00002/dist.jpg foi recusado a esta conta (403)"
            ),
        })
    );
    assert_eq!(again.stdout, format!("{FILM}\t0\t2\t1\timagens\n"));
    assert_eq!(asked_full(&fs, 2), 1);
    assert_eq!(listed.code, 0, "{}", listed.stderr);
    assert_eq!(listed.stdout, format!("{FILM}\t3\t1,3\t2\n"));

    let retried = env.run(&[&download[..], &["--tentar-pendentes"]].concat());
    assert_eq!(retried.stdout, format!("{FILM}\t0\t2\t1\timagens\n"));
    assert_eq!(asked_full(&fs, 2), 2);
}

#[test]
fn a_403_that_is_not_about_the_image_is_a_block_and_stops_the_download() {
    // A refusal that does not name the image, even in JSON.
    let fs = FakeServer::start(|request| match asked(request) {
        Asked::Full(2) => Reply::json(403, json!({"message": "Forbidden"}).to_string()),
        _ => open_film(4, request),
    });
    let env = TestEnv::logged_in(&fs);

    let blocked = env.run(&["imagens", "baixar", FILM, "--paralelo", "3"]);

    assert_eq!(blocked.code, 4, "{}", blocked.stderr);
    assert!(blocked.stderr.contains("bloqueio"), "{}", blocked.stderr);
    assert_eq!([2, 3, 4].map(|number| asked_full(&fs, number)), [1, 0, 0]);
    // The first image is kept, and the blocked one is not called restricted.
    let lines = manifest(&env.work_dir().join("imagens"));
    let kinds: Vec<_> = lines.iter().map(|line| line["situacao"].clone()).collect();
    assert_eq!(kinds, ["listado", "baixada"]);
}

#[test]
fn a_refused_signed_address_stops_the_film_download() {
    let fs = FakeServer::start(|request| match asked(request) {
        Asked::Signed(1) => Reply::empty(404),
        _ => open_film(2, request),
    });
    let env = TestEnv::logged_in(&fs);

    let refused = env.run(&["imagens", "baixar", FILM, "--paralelo", "1"]);
    assert_eq!(refused.code, 1, "{}", refused.stderr);
    assert!(refused.stderr.contains("404"), "{}", refused.stderr);
    assert!(
        refused.stderr.contains("endereço assinado"),
        "{}",
        refused.stderr
    );
    assert_eq!(asked_full(&fs, 2), 0);
}

#[test]
fn an_image_that_arrives_cut_short_is_left_pending_access_and_out_of_the_folder() {
    let fs = FakeServer::start(|request| match asked(request) {
        // A PNG that says its size and stops short of its pixels.
        Asked::Signed(1) => {
            let mut cut = encoded(GrayImage::new(40, 30), ImageFormat::Png);
            cut.truncate(cut.len() - 20);
            Reply::bytes(200, "image/png", cut)
        }
        // A JPEG without its end, which a lenient decoder would still open.
        Asked::Signed(2) => {
            let mut cut = page_of(2);
            cut.truncate(cut.len() - 100);
            Reply::bytes(200, "image/jpeg", cut)
        }
        _ => open_film(3, request),
    });
    let env = TestEnv::logged_in(&fs);

    let downloaded = env.run(&["imagens", "baixar", FILM]);

    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(downloaded.stdout, format!("{FILM}\t1\t0\t2\timagens\n"));
    let folder = env.work_dir().join("imagens");
    let lines = manifest(&folder);
    for cut in [1, 2] {
        assert_eq!(
            lines[cut]["motivo"], "o que veio não é uma imagem inteira",
            "image {cut}"
        );
    }
    // Only the whole image is in the folder, beside the manifest and its lock.
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 3);
}

#[test]
fn two_downloads_into_one_folder_take_turns_and_both_stay_in_the_manifest() {
    let fs = FakeServer::start(|request| {
        if let Asked::Signed(_) = asked(request) {
            thread::sleep(Duration::from_millis(200));
        }
        open_film(4, request)
    });
    let env = TestEnv::logged_in(&fs);

    let first = env
        .command(&["imagens", "baixar", FILM, "--ate", "2"])
        .spawn()
        .unwrap();
    let second = env.run(&["imagens", "baixar", FILM, "--de", "3"]);
    let first = crate::env::output(first);

    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(second.code, 0, "{}", second.stderr);
    let lines = manifest(&env.work_dir().join("imagens"));
    let numbers: Vec<_> = lines.iter().map(|line| line["numero"].clone()).collect();
    assert_eq!(
        numbers,
        [Value::Null, json!(1), json!(2), json!(3), json!(4)]
    );
}

#[test]
fn transfers_overlap_while_the_requests_to_familysearch_go_spaced_one_at_a_time() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let fs = FakeServer::start({
        let (in_flight, peak) = (Arc::clone(&in_flight), Arc::clone(&peak));
        move |request| {
            if let Asked::Signed(_) = asked(request) {
                let now = in_flight.fetch_add(1, Ordering::Relaxed) + 1;
                peak.fetch_max(now, Ordering::Relaxed);
                // A transfer outlasts the interval between two images.
                thread::sleep(Duration::from_millis(500));
                in_flight.fetch_sub(1, Ordering::Relaxed);
            }
            open_film(4, request)
        }
    });
    let env = TestEnv::new(&fs).with_var("GARIMPO_INTERVALO_IMAGENS_MS", "200");
    env.deliver_session();

    let downloaded = env.run(&["imagens", "baixar", FILM, "--paralelo", "3"]);

    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(downloaded.stdout, format!("{FILM}\t4\t0\t0\timagens\n"));
    assert!(
        peak.load(Ordering::Relaxed) >= 2,
        "the transfers overlapped"
    );
    let requests = fs.requests();
    let to_familysearch: Vec<_> = requests
        .iter()
        .filter(|request| matches!(asked(request), Asked::Full(_)))
        .collect();
    for pair in to_familysearch.windows(2) {
        let gap = pair[1].arrived - pair[0].arrived;
        assert!(gap >= Duration::from_millis(200), "{gap:?}");
    }
}

#[test]
fn thumbnails_are_downloaded_once_and_laid_out_on_a_contact_sheet() {
    let fs = FakeServer::start(|request| open_film(3, request));
    let env = TestEnv::logged_in(&fs);
    let thumbnails = ["imagens", "miniaturas", FILM, "--ate", "3"];

    let downloaded = env.run(&thumbnails);
    let requests_after_one_run = fs.requests().len();
    let again = env.run(&thumbnails);
    let names = [1, 2, 3].map(|number| format!("imagens/miniaturas/{FILM}_0000{number}.jpg"));
    let sheet = env.run(&[
        "imagens",
        "folha",
        &names[0],
        &names[1],
        &names[2],
        "--colunas",
        "2",
        "--saida",
        "folha.jpg",
    ]);

    assert_eq!(downloaded.code, 0, "{}", downloaded.stderr);
    assert_eq!(
        downloaded.stdout,
        format!("{FILM}\t3\t0\t0\timagens/miniaturas\n")
    );
    assert_eq!(
        fs::read(env.work_dir().join(&names[1])).unwrap(),
        page(22, 15)
    );
    assert_eq!(
        again.stdout,
        format!("{FILM}\t0\t3\t0\timagens/miniaturas\n")
    );
    assert_eq!(fs.requests().len(), requests_after_one_run);
    assert_eq!(sheet.code, 0, "{}", sheet.stderr);
    assert_eq!(
        sheet.stdout,
        format!(
            "1\t1\t{}\n1\t2\t{}\n2\t1\t{}\n",
            names[0], names[1], names[2]
        )
    );
    // Two cells of 256 px across and two down, with gaps of 8 px.
    let sheet = image::open(env.work_dir().join("folha.jpg")).unwrap();
    assert_eq!((sheet.width(), sheet.height()), (536, 536));

    // A sheet never takes the place of a file that exists, such as an image.
    let over = env.run(&["imagens", "folha", &names[0], "--saida", &names[1]]);
    assert_eq!(over.code, 2, "{}", over.stderr);
    assert_eq!(
        fs::read(env.work_dir().join(&names[1])).unwrap(),
        page(22, 15)
    );
}

/// Writes a synthetic page into the working directory of the environment.
fn page_file(env: &TestEnv, name: &str, bytes: &[u8]) {
    fs::write(env.work_dir().join(name), bytes).unwrap();
}

#[test]
fn a_page_is_cut_into_the_fewest_pieces_within_the_read_limit_and_the_original_stays() {
    let fs = FakeServer::start(|_| unreachable!("cropping makes no request"));
    let env = TestEnv::new(&fs);
    // Wider than the 2576 px the model reads.
    let original = page(3000, 200);
    page_file(&env, "pagina.jpg", &original);

    let cut = env.run(&["imagens", "recortar", "pagina.jpg"]);

    assert_eq!(cut.code, 0, "{}", cut.stderr);
    // Two pieces of 1564 px that share 128 px.
    assert_eq!(
        cut.stdout,
        "pagina.r0-0-1564x200.jpg\t0\t0\t1564\t200\n\
         pagina.r1436-0-1564x200.jpg\t1436\t0\t1564\t200\n"
    );
    let second = image::open(env.work_dir().join("pagina.r1436-0-1564x200.jpg")).unwrap();
    assert_eq!((second.width(), second.height()), (1564, 200));
    // The left edge of the second piece is the page at x = 1436: 1436 * 255 / 3000.
    let level = i32::from(second.to_luma8().get_pixel(0, 100).0[0]);
    assert!((level - 122).abs() <= 3, "{level}");
    assert_eq!(
        fs::read(env.work_dir().join("pagina.jpg")).unwrap(),
        original
    );
}

#[test]
fn a_region_is_cropped_and_enlarged_and_one_outside_the_page_is_refused() {
    let fs = FakeServer::start(|_| unreachable!("cropping makes no request"));
    let env = TestEnv::new(&fs);
    page_file(&env, "pagina.jpg", &page(1000, 400));

    let word = env.run(&[
        "imagens",
        "recortar",
        "pagina.jpg",
        "--regiao",
        "500,100,300,50",
        "--ampliar",
        "2",
    ]);
    let outside = env.run(&[
        "imagens",
        "recortar",
        "pagina.jpg",
        "--regiao",
        "900,0,200,50",
    ]);

    assert_eq!(word.code, 0, "{}", word.stderr);
    assert_eq!(
        word.stdout,
        "pagina.r500-100-300x50.a2.jpg\t500\t100\t300\t50\n"
    );
    let enlarged = image::open(env.work_dir().join("pagina.r500-100-300x50.a2.jpg")).unwrap();
    assert_eq!((enlarged.width(), enlarged.height()), (600, 100));
    // The middle of the piece is the page at x = 650: 650 * 255 / 1000.
    let level = i32::from(enlarged.to_luma8().get_pixel(300, 50).0[0]);
    assert!((level - 165).abs() <= 3, "{level}");
    assert_eq!(outside.code, 2, "{}", outside.stderr);
    assert!(outside.stderr.contains("1000 x 400"), "{}", outside.stderr);
}

#[test]
fn global_contrast_stretches_a_faded_page_to_the_full_range_of_grays() {
    let fs = FakeServer::start(|_| unreachable!("contrast makes no request"));
    let env = TestEnv::new(&fs);
    // A faded page: every gray between 100 and 150, lossless.
    let faded = GrayImage::from_fn(510, 20, |x, _| Luma([u8::try_from(100 + x / 10).unwrap()]));
    page_file(&env, "apagada.png", &encoded(faded, ImageFormat::Png));

    let stretched = env.run(&["imagens", "contraste", "apagada.png"]);

    assert_eq!(stretched.code, 0, "{}", stretched.stderr);
    assert_eq!(stretched.stdout, "apagada.contraste.jpg\n");
    let page = image::open(env.work_dir().join("apagada.contraste.jpg"))
        .unwrap()
        .to_luma8();
    let levels = |x: u32| i32::from(page.get_pixel(x, 10).0[0]);
    assert!(levels(2) <= 6, "{}", levels(2));
    assert!(levels(507) >= 249, "{}", levels(507));
    // The same change everywhere: the page still goes from dark to light.
    assert!((levels(255) - 127).abs() <= 8, "{}", levels(255));
}

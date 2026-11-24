//! What an image of a document may go through to be read: cropping,
//! enlarging, global contrast and a contact sheet. Nothing here draws,
//! erases or recomposes writing, and the original file is never changed:
//! each result is a new file whose name tells the operation.

use std::{
    fs::File,
    io::{self, BufWriter},
    path::{Path, PathBuf},
};

use anyhow::Context;
use image::{
    DynamicImage, GrayImage, ImageReader, Rgb, RgbImage,
    codecs::jpeg::JpegEncoder,
    imageops::{self, FilterType},
};
use serde_json::json;

use crate::{
    cli::{Config, Region},
    error::Error,
};

/// The longest side the model reads without shrinking the image, in pixels.
const READ_LIMIT: u32 = 2576;

/// How much two neighboring pieces of a cropped image share, in pixels, so a
/// line cut by one piece is whole in the other.
const OVERLAP: u32 = 128;

const JPEG_QUALITY: u8 = 90;

/// The side of one cell of a contact sheet and the gap between cells, in
/// pixels.
const CELL: u32 = 256;
const GAP: u32 = 8;

/// The share of the pixels left out at each end of the levels when the
/// contrast is stretched, so a few specks do not set the range.
const CLIPPED: u64 = 100;

fn open(file: &Path) -> Result<DynamicImage, Error> {
    let unreadable = |error: &dyn std::fmt::Display| {
        Error::Input(format!(
            "não consegui ler a imagem {}: {error}",
            file.display()
        ))
    };
    ImageReader::open(file)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|error| unreadable(&error))?
        .decode()
        .map_err(|error| unreadable(&error))
}

/// Writes a result as JPEG into `file`, as it was just opened.
fn save(image: &DynamicImage, file: &Path, opened: io::Result<File>) -> Result<(), Error> {
    let context = || format!("não consegui gravar {}", file.display());
    let writer = BufWriter::new(opened.with_context(context)?);
    image
        .write_with_encoder(JpegEncoder::new_with_quality(writer, JPEG_QUALITY))
        .with_context(context)?;
    Ok(())
}

/// The file of a result, beside the original: `<name>.<operation>.jpg`.
fn derived(file: &Path, operation: &str) -> PathBuf {
    file.with_extension(format!("{operation}.jpg"))
}

/// Where the fewest pieces no longer than `limit` start along `length`
/// pixels, and how long each one is. Neighbors share at least [`OVERLAP`].
fn pieces(length: u32, limit: u32) -> (Vec<u32>, u32) {
    if length <= limit {
        return (vec![0], length);
    }
    let count = (length - OVERLAP).div_ceil(limit - OVERLAP);
    let size = (length + (count - 1) * OVERLAP).div_ceil(count);
    let starts = (0..count)
        .map(|piece| piece * (length - size) / (count - 1))
        .collect();
    (starts, size)
}

/// Crops the image, or a region of it, into pieces that fit the read limit,
/// each one enlarged `enlarge` times.
pub(super) fn crop(
    config: &Config,
    file: &Path,
    region: Option<Region>,
    enlarge: u32,
) -> Result<(), Error> {
    let image = open(file)?;
    let (left, top, width, height) =
        region.map_or((0, 0, image.width(), image.height()), Region::parts);
    let inside = |start: u32, length: u32, side: u32| {
        start.checked_add(length).is_some_and(|end| end <= side)
    };
    if !inside(left, width, image.width()) || !inside(top, height, image.height()) {
        return Err(Error::Input(format!(
            "a região sai da imagem, que tem {} x {} pixels",
            image.width(),
            image.height()
        )));
    }
    // An enlarged piece has to fit the limit too.
    let limit = READ_LIMIT / enlarge;
    let (lefts, width) = pieces(width, limit);
    let (tops, height) = pieces(height, limit);
    for piece_top in &tops {
        for piece_left in &lefts {
            let (x, y) = (left + piece_left, top + piece_top);
            let mut piece = image.crop_imm(x, y, width, height);
            let mut operation = format!("r{x}-{y}-{width}x{height}");
            if enlarge > 1 {
                piece = piece.resize_exact(width * enlarge, height * enlarge, FilterType::Lanczos3);
                operation.push_str(&format!(".a{enlarge}"));
            }
            let output = derived(file, &operation);
            // The same crop of the same file gives the same piece, so an
            // earlier one is replaced.
            let piece = DynamicImage::ImageRgb8(piece.to_rgb8());
            save(&piece, &output, File::create(&output))?;
            config.print(
                &format!("{}\t{x}\t{y}\t{width}\t{height}", output.display()),
                &json!({"arquivo": output, "x": x, "y": y, "largura": width, "altura": height}),
            )?;
        }
    }
    Ok(())
}

/// The first level, in the order given, past the pixels left out.
fn edge(histogram: &[u64; 256], clipped: u64, mut levels: impl Iterator<Item = u8>) -> Option<u8> {
    let mut passed = 0;
    levels.find(|level| {
        passed += histogram[usize::from(*level)];
        passed > clipped
    })
}

/// Stretches the gray levels of the whole image to the full range: the same
/// change for every pixel, whatever part of the page it is in.
pub(super) fn contrast(config: &Config, file: &Path) -> Result<(), Error> {
    let mut gray: GrayImage = open(file)?.to_luma8();
    let mut histogram = [0u64; 256];
    for pixel in gray.pixels() {
        histogram[usize::from(pixel.0[0])] += 1;
    }
    let clipped = histogram.iter().sum::<u64>() / CLIPPED;
    let low = edge(&histogram, clipped, 0..=u8::MAX).unwrap_or(u8::MIN);
    let high = edge(&histogram, clipped, (0..=u8::MAX).rev()).unwrap_or(u8::MAX);
    // A flat image has no range to stretch.
    if high > low {
        let (low, range) = (u32::from(low), u32::from(high - low));
        for pixel in gray.pixels_mut() {
            let level = u32::from(pixel.0[0]).saturating_sub(low) * 255 / range;
            pixel.0[0] = u8::try_from(level).unwrap_or(u8::MAX);
        }
    }
    let output = derived(file, "contraste");
    save(
        &DynamicImage::ImageLuma8(gray),
        &output,
        File::create(&output),
    )?;
    config.print(&output.display().to_string(), &json!({"arquivo": output}))
}

/// Lays the images out, reduced, on one sheet, in the order given. Nothing
/// is written over them: the output tells which image is in which cell.
pub(super) fn sheet(
    config: &Config,
    files: &[PathBuf],
    output: &Path,
    columns: u32,
) -> Result<(), Error> {
    let span = |cells: u32| cells * CELL + (cells + 1) * GAP;
    // The sheet is always a new file: its name is the caller's, and it must
    // not take the place of an image.
    if output.exists() {
        return Err(Error::Input(format!(
            "{} já existe: dê outro nome à folha",
            output.display()
        )));
    }
    let rows = u32::try_from(files.len())
        .unwrap_or(u32::MAX)
        .div_ceil(columns);
    let most = (READ_LIMIT - GAP) / (CELL + GAP) * columns;
    if rows > most / columns {
        return Err(Error::Input(format!(
            "cabem no máximo {most} imagens numa folha de {columns} colunas: divida em mais folhas"
        )));
    }
    let mut sheet = RgbImage::from_pixel(span(columns), span(rows), Rgb([255, 255, 255]));
    for (file, position) in files.iter().zip(0u32..) {
        let small = open(file)?.thumbnail(CELL, CELL).to_rgb8();
        let (row, column) = (position / columns, position % columns);
        // Each image sits in the middle of its cell.
        let corner =
            |cell: u32, side: u32| i64::from(GAP + cell * (CELL + GAP) + (CELL - side) / 2);
        imageops::overlay(
            &mut sheet,
            &small,
            corner(column, small.width()),
            corner(row, small.height()),
        );
        config.print(
            &format!("{}\t{}\t{}", row + 1, column + 1, file.display()),
            &json!({"linha": row + 1, "coluna": column + 1, "arquivo": file}),
        )?;
    }
    save(
        &DynamicImage::ImageRgb8(sheet),
        output,
        File::create_new(output),
    )
}

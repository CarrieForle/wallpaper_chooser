use std::{cmp::*, error::Error, fmt::Debug, fs, io, path::{self, PathBuf}, sync::{Arc, LazyLock, atomic::{AtomicU32, Ordering}}};
use clap::{Parser};
use fraction::GenericFraction;
use terminal_size::terminal_size;
use image::{DynamicImage, LumaA, Pixel, Primitive};
use rayon::{ThreadPoolBuilder, prelude::*};
use regex::Regex;
use unicode_width::UnicodeWidthStr;

static REGEX_RATIO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+):([0-9]+)$").unwrap());
static REGEX_RATIO_RANGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9]+):([0-9]+)-([0-9]+):([0-9]+)$").unwrap());

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Folder path. Use current folder by default.
    #[arg(value_hint = clap::ValueHint::DirPath, default_value = "./")]
    directory: PathBuf,

    /// Do not recursively scan sub-folders.
    #[arg(short, long)]
    no_recursive: bool,

    /// The ratio of the image in one of the follow formats:
    ///   "X:Y" - image with exact ratio.
    ///   "X1:Y1-X2:Y2" - image in between two ratios.
    ///   "landscape" - image where its width is longer than height.
    ///   "portrait" - image where its height is longer than width.
    #[arg(short, long, value_parser = Dimension::parse, verbatim_doc_comment)]
    dimension: Option<Dimension>,

    /// Include images with inversed ratio. Apply when --dimension is specified in "X:Y" format; otherwise ignored.
    #[arg(short, long)]
    flipped: bool,

    /// Calculate brightness of image with alpha channel as if it has a white background, rather than black.
    #[arg(short = 'w', long)]
    alpha_as_white: bool,

    /// The number of threads to use. Chosen automatically if omitted.
    #[arg(short, long)]
    threads: Option<u16>,

    #[arg(long)]
    min_width: Option<u32>,

    #[arg(long)]
    max_width: Option<u32>,

    #[arg(long)]
    min_height: Option<u32>,

    #[arg(long)]
    max_height: Option<u32>,

    /// The minimum average brightness. A decimal from 0 (black) to 1 (white).
    #[arg(long)]
    min_brightness: Option<f64>,

    /// The maximum average brightness. A decimal from 0 (black) to 1 (white).
    #[arg(long)]
    max_brightness: Option<f64>,

    /// Do not print progress.
    #[arg(long)]
    no_progress: bool,
}

#[derive(Debug, Copy, Clone)]
struct Ratio(u32, u32);

#[derive(Debug, Copy, Clone)]
enum Dimension {
    Landscape,
    Portrait,
    Ratio(Ratio),
    RatioRange(Ratio, Ratio),
}

impl Dimension {
    fn parse(s: &str) -> Result<Self, Box<dyn Error + Send + Sync + 'static>> {
        match s {
            "landscape" => Ok(Dimension::Landscape),
            "portrait" => Ok(Dimension::Portrait),

            _ if let Some(cap) = REGEX_RATIO.captures(s) => {
                let (_, [x, y]) = cap.extract();
                let x = x.parse().expect("Ratio x should matches [0-9]");
                let y = y.parse().expect("Ratio y should matches [0-9]");

                Ok(Dimension::Ratio(Ratio(x, y)))
            }

            _ if let Some(cap) = REGEX_RATIO_RANGE.captures(s) => {
                let (_, [x1, y1, x2, y2]) = cap.extract();
                let x1 = x1.parse().expect("Ratio x1 should matches [0-9]");
                let y1 = y1.parse().expect("Ratio y1 should matches [0-9]");
                let x2 = x2.parse().expect("Ratio x2 should matches [0-9]");
                let y2 = y2.parse().expect("Ratio y2 should matches [0-9]");

                Ok(Dimension::RatioRange(Ratio(x1, y1), Ratio(x2, y2)))
            }
            _ => Err(Box::from("Invalid dimension"))
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(threads) = cli.threads {
        ThreadPoolBuilder::new().num_threads(threads as usize).build_global().unwrap();
    }

    let paths = scan(cli.directory, !cli.no_recursive)?;
    let file_count = paths.len();
    if !cli.no_progress {
        eprintln!("Scanning {} files.", file_count);
    }

    let counter = Arc::new(AtomicU32::new(1));
    let mut filtered_paths: Vec<_> = paths.into_par_iter().filter_map(|path| {
        let image = image::open(&path)
            .ok()?;

        if !cli.no_progress && let Some((width, _)) = terminal_size() {
            let progress = format!("{}/{} {}", Arc::clone(&counter).fetch_add(1, Ordering::Relaxed), file_count, path.display());
            eprint!("{}{}\r", progress, " ".repeat(max(width.0 as i32 - progress.width_cjk() as i32, 0) as usize));
        }

        let width = image.width();
        let height = image.height();

        if let Some(min_width) = cli.min_width && width < min_width {
            return None;
        }

        if let Some(max_width) = cli.max_width && width > max_width {
            return None;
        }

        if let Some(min_height) = cli.min_height && height < min_height {
            return None;
        }

        if let Some(max_height) = cli.max_height && height > max_height {
            return None;
        }

        if let Some(dimension) = cli.dimension
        {
            match dimension {
                Dimension::Landscape => {
                    if height > width {
                        return None;
                    }
                }

                Dimension::Portrait => {
                    if width > height {
                        return None;
                    }
                }

                Dimension::Ratio(Ratio(x, y)) => {
                    let ratio = GenericFraction::<u32>::new(x, y);
                    if !(ratio == GenericFraction::new(width, height) || 
                    cli.flipped && ratio == GenericFraction::new(height, width)) {
                        return None;
                    }
                }

                Dimension::RatioRange(ratio_low, ratio_high) => {
                    let mut ratio_low = GenericFraction::<u32>::new(ratio_low.0, ratio_low.1);
                    let mut ratio_high = GenericFraction::<u32>::new(ratio_high.0, ratio_high.1);
                    if ratio_low > ratio_high {
                        (ratio_low, ratio_high) = (ratio_high, ratio_low);
                    }

                    let image_ratio = GenericFraction::new(width, height);

                    if image_ratio > ratio_high || image_ratio < ratio_low {
                        return None;
                    }
                }
            }
        }

        if cli.min_brightness.is_some() || cli.max_brightness.is_some() {
            let brightness = avg_brightness(image, cli.alpha_as_white);
            let min_brightness = cli.min_brightness.unwrap_or(0f64);
            let max_brightness = cli.max_brightness.unwrap_or(1f64);

            if brightness < min_brightness || brightness > max_brightness {
                return None;
            }
        }

        Some(path)
    })
    .collect();

    filtered_paths.par_sort_unstable();
    for path in &filtered_paths {
        println!("{}", path.display());
    }

    if !cli.no_progress {
        eprintln!("Found {} images.", filtered_paths.len());
    }
    
    Ok(())
}

fn avg_brightness(image: DynamicImage, alpha_as_white: bool) -> f64
{
    let grayscale_image = image.into_luma_alpha8();
    const WHITE: u8 = <LumaA::<u8> as Pixel>::Subpixel::DEFAULT_MAX_VALUE;
    const BLACK: u8 = <LumaA::<u8> as Pixel>::Subpixel::DEFAULT_MIN_VALUE;
    let background_color = if alpha_as_white { 
        WHITE 
    } else { 
        BLACK 
    };

    let mut luma_sum: u32 = 0;

    for (_, _, pixel) in grayscale_image.enumerate_pixels() {
        let [luma, alpha] = pixel.0;
        let new_luma = luma as i16 + ((background_color as i16 - luma as i16) as f64 * (1f64 - alpha as f64 / WHITE as f64)) as i16;
        let new_luma = min(max(BLACK as i16, new_luma), WHITE as i16) as u32;
        luma_sum += new_luma;
    }

    luma_sum as f64 / (grayscale_image.width() * grayscale_image.height() * WHITE as u32) as f64
}

fn scan(dir: PathBuf, recursive: bool) -> io::Result<Vec<PathBuf>>
{
    let paths = fs::read_dir(dir)?;
    let mut res = Vec::new();
    for entry in paths
    {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            res.push(path::absolute(path)?);
        } else if recursive {
            res.append(&mut scan(path, recursive)?)
        }
    }

    Ok(res)
}
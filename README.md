# Wallpaper Chooser

An images filter CLI that allows you to filter by:
- height and width
- ratio
- brightness

This is a personal tool to choose a wallpaper from around 6,000 of images in my computer. I use this to roughly filter down to few hundreds, and then choose my favorite by eyes. 

# Speed

**I did not do a proper benchmark**. However for my use case of 6,364 files comprised of mostly jpg and png, less than 10 gifs and around 300 non-image files, taking a total of 8.27 GiB of space, it spent 37 seconds with this command:

```
.\wallpaper-chooser --brightness 0.35
```

# Usage

```powershell
.\wallpaper_chooser.exe -h
```

```
wallpaper_chooser.exe [OPTIONS] [DIRECTORY]

Arguments:
  [DIRECTORY]  Folder path. Use current folder by default [default: ./]

Options:
  -n, --no-recursive
          Do not recursively scan sub-folders
  -d, --dimension <DIMENSION>
          The ratio of the image in one of the follow formats:
            "X:Y" - image with exact ratio.
            "X1:Y1-X2:Y2" - image in between two ratios.
            "landscape" - image where its width is longer than height.
            "portrait" - image where its height is longer than width.
  -f, --flipped
          Include images with inversed ratio. Apply when --dimension is specified in "X:Y" format; otherwise ignored
  -w, --alpha-as-white
          Calculate brightness of image with alpha channel as if it has a white background, rather than black
  -t, --threads <THREADS>
          The number of threads to use. Chosen automatically if omitted
      --min-width <MIN_WIDTH>
          
      --max-width <MAX_WIDTH>
          
      --min-height <MIN_HEIGHT>
          
      --max-height <MAX_HEIGHT>
          
      --min-brightness <MIN_BRIGHTNESS>
          The minimum average brightness. A decimal from 0 (black) to 1 (white)
      --max-brightness <MAX_BRIGHTNESS>
          The maximum average brightness. A decimal from 0 (black) to 1 (white)
      --no-progress
          Do not print progress
  -h, --help
          Print help
  -V, --version
          Print version
```

## Note

By default the program will max out CPU usage for maximum speed and might break down other processes. You can pass `-t <number_of_thread>` to limit CPU usage.

# Download

Only Windows executable is provided. See [Releases](https://github.com/CarrieForle/wallpaper_chooser/releases).

For other systems you need to compile the codes. See [Build](#Build).

# Build

Rust 2024 edition is required.

```sh
cargo build --release
```

## Run

To run the code:

```sh
cargo run
```
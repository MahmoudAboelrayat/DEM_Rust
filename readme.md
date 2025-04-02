# Elevation Data Processing and Visualization

## Overview
This Rust project processes elevation data from ASC files, converts it into grayscale and RGB images, and generates hillshade visualizations. The tool provides functionalities to read ASC files, extract elevation data, normalize it, and apply grayscale or color gradients for visualization.

## Features
- **Reads ASC elevation files**: Extracts elevation data and metadata.
- **Converts elevation data to images**:
  - Grayscale representation
  - RGB representation using a color gradient
- **Generates hillshade images**:
  - Grayscale hillshade
  - RGB hillshade
- **Timestamps output images** for versioning.

## Dependencies
This project relies on several Rust libraries:
- `image` - For image processing and saving outputs.
- `colorgrad` - For applying color gradients.
- `chrono` - For timestamping output files.
- `anyhow` - For error handling.
- `std::fs` - For file operations.

## Installation
1. Ensure you have Rust installed: [Rust Installation](https://www.rust-lang.org/tools/install)
2. Clone this repository:
   ```sh
   git https://github.com/MahmoudAboelrayat/DEM_Rust.git
   cd DEM_Rust
   ```
3. Build the project:
   ```sh
   cargo build 
   ```
4. Download the dataset

## Usage
Run the program by providing an ASC file path:
```sh
cargo run -- path/to/elevation.asc
```
If no path is provided, the program uses a default ASC file with the location as the variable file_path.

**Note:** If you want to use the current code, you need to place the dataset in the specified folder.

## Output Files
- `output_YYYYMMDD_HHMMSS.png` - Grayscale elevation image
- `output_rgb_YYYYMMDD_HHMMSS_turbo.png` - RGB elevation image
- `hillshade_gray_YYYYMMDD_HHMMSS.png` - Grayscale hillshade
- `hillshade_rgb_YYYYMMDD_HHMMSS.png` - RGB hillshade

All output images are saved in the `src/output_img` directory.


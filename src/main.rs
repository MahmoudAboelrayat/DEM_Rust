use std::fs::File;
use std::error::Error;
use colorgrad::{Gradient, preset};
use image::{DynamicImage, Luma, Rgba, RgbaImage, GrayImage};
use anyhow::Result;
use std::io::Read;
use chrono::Local;
// use show_image::{create_window, ImageInfo, ImageView, WindowOptions, WindowProxy};
// use walkdir::WalkDir;
// use std::path::PathBuf;
// use std::env;
// use std::io::{BufReader,BufRead};
// use clap::{Parser, Subcommand};
// use image::io::Reader as ImageReader;


/// Reads the content of a file and returns it as a string.
fn read_file(file_path: &str) -> String {
    // Read a file in the local file system
    let mut data_file = File::open(file_path).unwrap();
    
    // Create an empty mutable string
    let mut file_content = String::new();
    
    // Copy contents of file to a mutable string
    data_file.read_to_string(&mut file_content).unwrap();
    return file_content;
}


/// Parses an ASC file content into elevation data, width, and height.
fn asc_to_image(content: String) -> Result<(Vec<f32>, u32, u32), Box<dyn Error>> {
    let mut header_lines = 6;
    let mut width = 0;
    let mut height = 0;
    let mut data_elevation = Vec::new();
    let mut nodata_value =f32::NAN;

    let mut reader = content.lines();
    while let Some(line) = reader.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if header_lines>0 {
            header_lines -= 1;
            match parts.as_slice() {
                ["ncols", ncols] => width = ncols.parse::<u32>()?,                
                ["nrows", nrows] => height = nrows.parse::<u32>()?,
                ["nodata_value", nodata] => nodata_value = nodata.parse::<f32>()?,
            _ => {}
            }
        } else {
            // Read the elevation data
            for part in line.split_whitespace() {
                if let Ok(value) = part.parse::<f32>() {
                    // Check if the value is equal to the nodata_value
                    // and push it as NaN if it is
                    // Otherwise, push the value as is
                    data_elevation.push(if value == nodata_value {f32::NAN} else {value});
                }
            }
        }
    }
    Ok((data_elevation, width, height))
}

/// Converts elevation data into a grayscale image.
fn data_to_grayscale(data_processed: Vec<f32>, width: u32, height: u32) -> GrayImage {
    let mut image = GrayImage::new(width, height);
    let min_val = data_processed.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = data_processed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = max_val - min_val;

    for (i, &value) in data_processed.iter().enumerate() {
        let x = (i % width as usize) as u32;
        let y = (i / width as usize) as u32;
        let normalized_value = if range > 0.0 { (value - min_val) / range } else { 0.0 };
        let pixel_value = (normalized_value * 255.0) as u8;
        image.put_pixel(x, y, Luma([pixel_value]));
    }
    image
}

/// Converts elevation data into an RGB image using a color gradient.
fn rgb(data_processed: Vec<f32>, width: u32, height: u32) -> RgbaImage {
    let mut image = RgbaImage::new(width, height);
    let min_val = data_processed.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_val = data_processed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = max_val - min_val;
    let gradient = preset::turbo();

    for (i, &value) in data_processed.iter().enumerate() {
        let x = (i % width as usize) as u32;
        let y = (i / width as usize) as u32;
        let normalized_value = if range > 0.0 { (value - min_val) / range } else { 0.0 };
        let color = gradient.at(normalized_value);
        let [r, g, b, _] = color.to_rgba8();
        image.put_pixel(x, y, Rgba([r, g, b, 255]));
    }
    image
}

/// Generates hillshade images (grayscale and RGB) from elevation data.
fn hill_shading(data: &Vec<f32>, colored_image:RgbaImage, width: u32, height: u32, azimuth: f32, altitude: f32) -> (GrayImage, RgbaImage) {
    let mut shaded_image = GrayImage::new(width, height);
    let mut shaded_image_rgb = RgbaImage::new(width, height);
    let radians = std::f32::consts::PI / 180.0;
    let azimuth_rad = azimuth * radians;
    let altitude_rad = altitude * radians;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = |dx: i32, dy: i32| ((y as i32 + dy) * width as i32 + (x as i32 + dx)) as usize;

            let z1 = data[idx(-1, -1)];
            let z2 = data[idx(0, -1)];
            let z3 = data[idx(1, -1)];
            let z4 = data[idx(-1, 0)];
            let z5 = data[idx(0, 0)];  // Center pixel
            let z6 = data[idx(1, 0)];
            let z7 = data[idx(-1, 1)];
            let z8 = data[idx(0, 1)];
            let z9 = data[idx(1, 1)];

            let dz_dx = (z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7);
            let dz_dy = (z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3);

            let slope = (dz_dx.powi(2) + dz_dy.powi(2)).sqrt().atan();
            let aspect = dz_dy.atan2(dz_dx);

            let intensity = 255.0 * (
                altitude_rad.cos() * slope.cos() +
                altitude_rad.sin() * slope.sin() * (azimuth_rad - aspect).cos()
            );

            let pixel_value = intensity.clamp(0.0, 255.0) as u8;
            shaded_image.put_pixel(x, y, Luma([pixel_value]));

            let color = colored_image.get_pixel(x, y);
            let r  = (color[0] as f32 * pixel_value as f32 / 255.0) as u8;
            let g  = (color[1] as f32 * pixel_value as f32 / 255.0) as u8;
            let b  = (color[2] as f32 * pixel_value as f32 / 255.0) as u8;

            shaded_image_rgb.put_pixel(x, y, Rgba([r,g,b, 255]));
        }
    }

    // return the shaded image and the RGB image
    (shaded_image, shaded_image_rgb)
}


fn main() {

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_image>", args[0]);
        std::process::exit(1);
    }
    println!("Reading file path: {}", args[1]);
    let file_content = read_file(&args[1]);

    // use the asc_to_image function to open the file
    let (data_elevation, width, height) = asc_to_image(file_content).expect("Failed to read ASC file"); 
    println!("Width: {:?}", width);
    println!("Height: {:?}", height);
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Generate grayscale image
    let image_gray = data_to_grayscale(data_elevation.clone(), width, height);
    let filename_gray = format!("output_img/output_{}.png", timestamp);
    image_gray.save(&filename_gray).expect("Failed to save image");
    print!("Image saved as output.png\n");
    

    // Generate RGB image
    let img_rgb = rgb(data_elevation.clone(), width, height);
    
    DynamicImage::ImageRgba8(img_rgb.clone())
        .save( format!("output_img/output_rgb_{}_turbo.png", timestamp))
        .expect("Failed to save image");
    print!("Image saved as output_rgb.png\n");

    // create a hillshade image 
    let (hillshade_gray, hillshade_rgb) = hill_shading(&data_elevation, img_rgb, width, height, 315.0, 45.0);
    
    //  save the hillshade images
    DynamicImage::ImageLuma8(hillshade_gray)
        .save(format!("output_img/hillshade_gray_{}.png", timestamp))
        .expect("Failed to save image");
    print!("Hillshade image saved as hillshade_gray.png\n");
    
    // save the hillshade image in RGB
    DynamicImage::ImageRgba8(hillshade_rgb)
        .save(format!("output_img/hillshade_rgb_{}.png", timestamp))
        .expect("Failed to save image");
    print!("Hillshade image saved as hillshade_rgb.png\n");

}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader, BufRead};

    #[test]
    fn test_asc_to_image() {
        let file_path = "/home/anas/Downloads/0925_6225/LITTO3D_FRA_0929_6224_20150529_LAMB93_RGF93_IGN69/MNT1m/LITTO3D_FRA_0929_6224_MNT_20150128_LAMB93_RGF93_IGN69.asc";
        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);
        let content: String = reader.lines().filter_map(Result::ok).collect::<Vec<_>>().join("\n");
        let (data_elevation, width, height) = asc_to_image(content).expect("Failed to read ASC file");
        assert_eq!(width, 1000);
        assert_eq!(height, 1000);
        assert_eq!(data_elevation.len(), (width * height) as usize);
    }
}

//  test the data_to_grayscale function
mod grayscale {
    use super::*;

    #[test]
    fn test_data_to_grayscale() {
        let data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let width = 5;
        let height = 1;
        let image = data_to_grayscale(data.clone(), width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
    }
}
use std::fs::File;
use std::error::Error;
use colorgrad::{Gradient, preset};
use image::{DynamicImage, Luma, Rgba, RgbaImage, GrayImage};
use anyhow::Result;
use std::io::Read;
use chrono::Local;
use imageproc::drawing::draw_line_segment_mut;
use std::f32::consts::PI;

/// Reads the content of a file and returns it as a string.
/// # Arguments
/// * `file_path` - A string representing path to the file.
///
/// # Returns
/// * A `String` containing the content of the file.
///
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
/// Arguments
/// * `content` - A string containing the content of the ASC file.
/// Returns a tuple containing the elevation data as a vector of f32, width, height, and cell size.
fn asc_to_image(content: String) -> Result<(Vec<f32>, u32, u32,f32), Box<dyn Error>> {
    let mut header_lines = 6;
    let mut width = 0;
    let mut height = 0;
    let mut data_elevation = Vec::new();
    let mut nodata_value =f32::NAN;
    let mut cell_size = 1.0;

    let mut reader = content.lines();
    while let Some(line) = reader.next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if header_lines>0 {
            header_lines -= 1;
            match parts.as_slice() {
                ["ncols", ncols] => width = ncols.parse::<u32>()?,                
                ["nrows", nrows] => height = nrows.parse::<u32>()?,
                ["nodata_value", nodata] => nodata_value = nodata.parse::<f32>()?,
                ["cellsize", cellsize]=> cell_size = cellsize.parse::<f32>()?,
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
    Ok((data_elevation, width, height,cell_size))
}

/// Converts elevation data into a grayscale image.
/// # Arguments
/// * `data_processed` - A vector of f32 representing the elevation data.
/// * `width` - The width of the image.
/// * `height` - The height of the image.
/// # Returns
/// * A `GrayImage` object representing the grayscale image.

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
/// # Arguments
/// * `data_processed` - A vector of f32 representing the elevation data.
/// * `width` - The width of the image. 
/// * `height` - The height of the image.
/// # Returns
/// * A `RgbaImage` object representing the RGB image.
/// The function uses a color gradient to map the elevation data to RGB colors.
/// The gradient is generated using the `colorgrad` crate.
/// The function normalizes the elevation data to the range [0, 1] and then maps it to RGB colors.
/// The function uses the `turbo` gradient from the `colorgrad` crate.
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
/// # Arguments
/// * `data` - A vector of f32 representing the elevation data.
/// * `colored_image` - A `RgbaImage` object representing the colored image.
/// * `width` - The width of the image.
/// * `height` - The height of the image.
/// * `cellsize` - The size of each cell in the elevation data.
/// * `azimuth` - The azimuth angle for the light source.       
/// * `altitude` - The altitude angle for the light source.
/// # Returns     
/// * A tuple containing two images: the grayscale hillshade image and the RGB hillshade image.
/// The function calculates the slope and aspect of the terrain using the hillshading algorithm introduced in:
/// https://pro.arcgis.com/en/pro-app/latest/tool-reference/3d-analyst/how-hillshade-works.htm
fn hill_shading(data: &Vec<f32>, colored_image:RgbaImage, width: u32, height: u32, cellsize: f32, azimuth: f32, altitude: f32) -> (GrayImage, RgbaImage) {
    let mut shaded_image = GrayImage::new(width, height);
    let mut shaded_image_rgb: image::ImageBuffer<Rgba<u8>, Vec<u8>> = RgbaImage::new(width, height);
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
            // let z5 = data[idx(0, 0)];  // Center pixel
            let z6 = data[idx(1, 0)];
            let z7 = data[idx(-1, 1)];
            let z8 = data[idx(0, 1)];
            let z9 = data[idx(1, 1)];

            let dz_dx = ((z3 + 2.0 * z6 + z9) - (z1 + 2.0 * z4 + z7)) / (8.0 * cellsize);
            let dz_dy = ((z7 + 2.0 * z8 + z9) - (z1 + 2.0 * z2 + z3)) / (8.0 * cellsize);

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

fn draw_vector_field(image: &mut RgbaImage, gradients: &Vec<(f32, f32)>, width: u32, height: u32) {
    let arrow_color = Rgba([255, 255, 255, 255]); // Red color
    let step = 30;  // ⬆ Increase spacing (fewer arrows)
    let arrow_length = step as f32 * 0.8; // ⬆ Make arrows bigger
    let arrowhead_size = step as f32 * 0.5; // ⬆ Increase arrowhead size

    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let idx = (y * width + x) as usize;

            let (dx, dy) = gradients[idx];
            let norm = (dx.powi(2) + dy.powi(2)).sqrt();

            if norm > 0.0 {
                let scale = arrow_length / norm; // Normalize arrow size
                let end_x = x as f32 + scale * dx;
                let end_y = y as f32 + scale * dy;

                // Draw the main arrow line
                draw_line_segment_mut(image, (x as f32, y as f32), (end_x, end_y), arrow_color);

                // Compute arrowhead directions (-30° and +30°)
                let angle = dy.atan2(dx); // Compute direction
                let left_angle = angle + (PI / 6.0);  // +30°
                let right_angle = angle - (PI / 6.0); // -30°

                let left_x = end_x - arrowhead_size * left_angle.cos();
                let left_y = end_y - arrowhead_size * left_angle.sin();
                let right_x = end_x - arrowhead_size * right_angle.cos();
                let right_y = end_y - arrowhead_size * right_angle.sin();

                // Draw the arrowhead lines
                draw_line_segment_mut(image, (end_x, end_y), (left_x, left_y), arrow_color);
                draw_line_segment_mut(image, (end_x, end_y), (right_x, right_y), arrow_color);
            }
        }
    }
}
fn compute_gradients(data: &Vec<f32>, width: u32, height: u32, window_size: u32) -> Vec<(f32, f32)> {
    let mut gradients = vec![(0.0, 0.0); (width * height) as usize];
    
    // Ensure window_size is odd for symmetry
    let half_window = window_size / 2;
    
    for y in half_window..(height - half_window) {
        for x in half_window..(width - half_window) {
            let idx = (y * width + x) as usize;
            
            // Compute dz/dx using the specified window size
            let mut dz_dx = 0.0;
            for offset in 1..=half_window as i32 {
                // Positive contribution from left points
                dz_dx += data[(y * width + (x as i32 - offset) as u32) as usize];
                // Negative contribution from right points
                dz_dx -= data[(y * width + (x as i32 + offset) as u32) as usize];
            }
            dz_dx /= half_window as f32;
            
            // Compute dz/dy using the specified window size
            let mut dz_dy = 0.0;
            for offset in 1..=half_window as i32 {
                // Positive contribution from top points
                dz_dy += data[((y as i32 - offset) as u32 * width + x) as usize];
                // Negative contribution from bottom points
                dz_dy -= data[((y as i32 + offset) as u32 * width + x) as usize];
            }
            dz_dy /= half_window as f32;
            
            gradients[idx] = (dz_dx, dz_dy);
        }
    }
    
    gradients
}




fn main() {
    let output_path = "src/output_img";
    let mut file_path = "/home/anas/Downloads/0925_6225/LITTO3D_FRA_0925_6225_20150529_LAMB93_RGF93_IGN69/MNT1m/LITTO3D_FRA_0925_6225_MNT_20150529_LAMB93_RGF93_IGN69.asc";
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("No file path provided. Using default:");
        // std::process::exit(1);
    }
    else{
        println!("Reading file path: {}", args[1]);
        file_path = args[1].as_str();
    }
    let file_content = read_file(file_path);

    // use the asc_to_image function to open the file
    let (data_elevation, width, height,cell_size) = asc_to_image(file_content).expect("Failed to read ASC file"); 
    println!("Width: {:?}", width);
    println!("Height: {:?}", height);
    
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    
    // Generate grayscale image
    let image_gray = data_to_grayscale(data_elevation.clone(), width, height);
    let filename_gray = format!("{}/output_{}.png", output_path,timestamp);
    image_gray.save(&filename_gray).expect("Failed to save image");
    print!("Image saved as output.png\n");
    

    // Generate RGB image
    let img_rgb = rgb(data_elevation.clone(), width, height);
    
    DynamicImage::ImageRgba8(img_rgb.clone())
        .save( format!("{}/output_rgb_{}_turbo.png", output_path,timestamp))
        .expect("Failed to save image");
    print!("Image saved as output_rgb.png\n");

    // create a hillshade image 
    let (hillshade_gray, hillshade_rgb) = hill_shading(&data_elevation, img_rgb.clone(), width, height,cell_size,315.0, 45.0);
    
    //  save the hillshade images
    DynamicImage::ImageLuma8(hillshade_gray)
        .save(format!("{}/hillshade_gray_{}.png",output_path, timestamp))
        .expect("Failed to save image");
    print!("Hillshade image saved as hillshade_gray.png\n");
    
    // save the hillshade image in RGB
    DynamicImage::ImageRgba8(hillshade_rgb.clone())
        .save(format!("{}/hillshade_rgb_{}.png",output_path, timestamp))
        .expect("Failed to save image");
    print!("Hillshade image saved as hillshade_rgb.png\n");

    let mut grad_img = hillshade_rgb.clone();
    let gradients = compute_gradients(&data_elevation.clone(), width, height,61);
    draw_vector_field(&mut grad_img, &gradients, width, height);
    
    DynamicImage::ImageRgba8(grad_img)
    .save(format!("{}/hillshade_grasssd_img_{}.png",output_path, timestamp))
    .expect("Failed to save image");
    print!("Hillshade image saved as hillshade_grad_img.png\n");

}


#[cfg(test)]
mod tests {
    use imageproc::gradients;

    use super::*;
    use std::fs;
    // use std::io::{BufReader, BufRead};

    fn create_dummy_asc_file(content: &str) -> String {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("dummy.asc");
        fs::write(&file_path, content).expect("Failed to write dummy ASC file");
        file_path.to_str().unwrap().to_string()
    }

    #[test]
    /// Test the read_file function 
    /// It creates a dummy ASC file and checks if the content is read correctly.
    /// It also cleans up the dummy file after the test.

    fn test_read_file_success() {
        let content = "This is a test file.";
        let file_path = create_dummy_asc_file(content);
        let result = read_file(&file_path);
        assert_eq!(result, content);
        fs::remove_file(&file_path).unwrap();
    }

    #[test]
    /// Given a know data set, it checks if the asc_to_image function parses the data correctly.
    fn test_asc_to_image_valid() {
        let content = "ncols 5\nnrows 2\nxllcorner 0\nyllcorner 0\ncellsize 1\nnodata_value -9999\n1 2 3 4 5\n6 7 8 9 10\n";
        let result = asc_to_image(content.to_string());
        assert!(result.is_ok());
        let (data, width, height, cellsize) = result.unwrap();
        assert_eq!(width, 5);
        assert_eq!(height, 2);
        assert_eq!(data.len(), 10);
        assert_eq!(data, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]);
    }

    #[test]
    /// It checks that asc_to_image handle the nodata_value correctly.
    fn test_asc_to_image_with_nodata() {
        let content = "ncols 3\nnrows 2\nxllcorner 0\nyllcorner 0\ncellsize 1\nnodata_value -9999\n1 2 -9999\n-9999 5 6\n";
        let result = asc_to_image(content.to_string());
        assert!(result.is_ok());
        let (data, width, height, cellsize) = result.unwrap();
        assert_eq!(width, 3);
        assert_eq!(height, 2);
        assert_eq!(data.len(), 6);
        assert!(data[2].is_nan());
        assert!(data[3].is_nan());
        assert_eq!(data[0], 1.0);
        assert_eq!(data[1], 2.0);
        assert_eq!(data[4], 5.0);
        assert_eq!(data[5], 6.0);
    }

    #[test]
    /// It checks that the function returns an error when the header is invalid.
    fn test_asc_to_image_invalid_header() {
        let content = "ncols abc\nnrows 2\nxllcorner 0\nyllcorner 0\ncellsize 1\nnodata_value -9999\n1 2 3\n4 5 6\n";
        let result = asc_to_image(content.to_string());
        assert!(result.is_err());
    }

    #[test]
    /// It checks if the fucntion data_to_grayscale maps the data correctly to grayscale.
    fn test_data_to_grayscale_basic() {
        let data = vec![0.0, 1.0, 2.0];
        let width = 3;
        let height = 1;
        let image = data_to_grayscale(data, width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
        assert_eq!(image.get_pixel(0, 0), &Luma([0]));
        assert_eq!(image.get_pixel(1, 0), &Luma([127])); 
        assert_eq!(image.get_pixel(2, 0), &Luma([255]));
    }

    #[test]
    /// It checks how the function data_to_grayscale handles NaN values.
    /// It should map NaN values to the minimum value of the grayscale range.
    fn test_data_to_grayscale_with_nan() {
        let data = vec![0.0, f32::NAN, 2.0];
        let width = 3;
        let height = 1;
        let image = data_to_grayscale(data, width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
        assert_eq!(image.get_pixel(0, 0), &Luma([0]));
        assert_eq!(image.get_pixel(1, 0), &Luma([0])); // NaN should result in min value
        assert_eq!(image.get_pixel(2, 0), &Luma([255]));
    }

    #[test]
    /// It checks if the function data_to_grayscale handles constant values correctly.
    fn test_data_to_grayscale_constant_value() {
        let data = vec![5.0, 5.0, 5.0];
        let width = 3;
        let height = 1;
        let image = data_to_grayscale(data, width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
        assert_eq!(image.get_pixel(0, 0), &Luma([0]));
        assert_eq!(image.get_pixel(1, 0), &Luma([0]));
        assert_eq!(image.get_pixel(2, 0), &Luma([0]));
    }

    #[test]
    /// It checks if the rgb function maps the data correctly to RGB image with width and height.
    fn test_rgb_basic() {
        let data = vec![0.0, 1.0, 2.0];
        let width = 3;
        let height = 1;
        let image = rgb(data, width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);

    }

    #[test]
    /// It checks if the rgb function handles NaN values correctly.
    fn test_rgb_with_nan() {
        let data = vec![0.0, f32::NAN, 2.0];
        let width = 3;
        let height = 1;
        let image = rgb(data, width, height);
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), height);
    }

    #[test]
    /// It checks if the hill_shading maps the data correctly to gray scale and RGB image with width and height.
    fn test_hill_shading_basic() {
        let data = vec![
            1.0, 1.0, 1.0,
            1.0, 2.0, 1.0,
            1.0, 1.0, 1.0,
        ];
        let width = 3;
        let height = 3;
        let cellsize = 1.0;
        let colored_image = RgbaImage::new(width, height);
        let (shaded_gray, shaded_rgb) = hill_shading(&data, colored_image, width, height, cellsize, 315.0, 45.0);
        assert_eq!(shaded_gray.width(), width);
        assert_eq!(shaded_gray.height(), height);
        assert_eq!(shaded_rgb.width(), width);
        assert_eq!(shaded_rgb.height(), height);

    }

    #[test]
    /// Checks if the hill_shading function handles NaN values correctly.
    fn test_hill_shading_with_nan() {
        let data = vec![
            1.0, 1.0, 1.0,
            1.0, f32::NAN, 1.0,
            1.0, 1.0, 1.0,
        ];
        let width = 3;
        let height = 3;
        let cellsize = 1.0;
        let colored_image = RgbaImage::new(width, height); // Dummy colored image
        let (shaded_gray, shaded_rgb) = hill_shading(&data, colored_image, width, height, cellsize, 315.0, 45.0);
        assert_eq!(shaded_gray.width(), width);
        assert_eq!(shaded_gray.height(), height);
        assert_eq!(shaded_rgb.width(), width);
        assert_eq!(shaded_rgb.height(), height);
        // Check if the center pixel (affected by NaN neighbor) is black
        // assert_eq!(shaded_gray.get_pixel(1, 1), &Luma([0]));
        assert_eq!(shaded_rgb.get_pixel(1, 1), &Rgba([0, 0, 0, 255]));
    }

    #[test]
    /// It checks if the hill_shading function handles edge cases correctly.
    fn test_hill_shading_edge_cases() {
        let data = vec![
            1.0, 2.0,
            3.0, 4.0,
        ];
        let width = 2;
        let height = 2;
        let cellsize = 1.0;
        let colored_image = RgbaImage::new(width, height);
        let (shaded_gray, shaded_rgb) = hill_shading(&data, colored_image, width, height,cellsize, 315.0, 45.0);
        assert_eq!(shaded_gray.width(), width);
        assert_eq!(shaded_gray.height(), height);
        assert_eq!(shaded_rgb.width(), width);
        assert_eq!(shaded_rgb.height(), height);
        assert_eq!(shaded_gray.get_pixel(0, 0), &Luma([0]));
        assert_eq!(shaded_gray.get_pixel(1, 0), &Luma([0]));
        assert_eq!(shaded_gray.get_pixel(0, 1), &Luma([0]));
        assert_eq!(shaded_gray.get_pixel(1, 1), &Luma([0]));
        assert_eq!(shaded_rgb.get_pixel(1, 0), &Rgba([0, 0, 0, 0]));
        assert_eq!(shaded_rgb.get_pixel(0, 0), &Rgba([0, 0, 0, 0]));
        assert_eq!(shaded_rgb.get_pixel(0, 1), &Rgba([0, 0, 0, 0]));
        assert_eq!(shaded_rgb.get_pixel(1, 1), &Rgba([0, 0, 0, 0]));
    }
}
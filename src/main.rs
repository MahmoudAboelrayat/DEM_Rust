use std::{cell, fs::File};
use std::io::Read;
use image::{RgbImage, Luma};
use colorgrad::Gradient;

fn read_file(file_path: &str) -> String {
    // Read a file in the local file system
    let mut data_file = File::open(file_path).unwrap();

    // Create an empty mutable string
    let mut file_content = String::new();

    // Copy contents of file to a mutable string
    data_file.read_to_string(&mut file_content).unwrap();
    return file_content;
}

fn get_data(file_data: String) -> (usize, usize, Vec<f64>){
    // Open the file

    let mut ncols = 0;
    let mut nrows = 0;
    let mut data = Vec::new();
    let lines = file_data.split('\n');
    // Iterate over each line in the file
    for line in lines{
        let elements: Vec<&str> = line.split_whitespace().collect();

        if elements.is_empty() {
            continue;
        }
        if elements[0] == "ncols" {
            ncols = elements[1].parse::<usize>().unwrap();
        } 
        else if elements[0] == "nrows" {
            nrows = elements[1].parse::<usize>().unwrap();
        } 
        else if elements[0] == "xllcenter" || elements[0] == "yllcenter" || 
                  elements[0] == "cellsize" || elements[0] == "nodata_value" {
            continue;
        } else {
            for value in elements {
                if let Ok(num) = value.parse::<f64>() {
                    data.push(num);
                }
            }
        }
    }
    (ncols, nrows, data)}


fn plot_data(nrows:usize,ncols:usize,data:Vec<f64>){
    // let shadowed_data = hellshade(data, nrows, ncols, 90.0, 315.0, 1.0, 1.0);
    let colors = data_to_color(&data);
    let shadows = add_shadow(&data, ncols, nrows, &colors);
    let img = RgbImage::from_vec(ncols as u32, nrows as u32, shadows).unwrap();
    img.save("output.png").unwrap();
    println!("Image saved as output.png");
}

fn data_to_color(data: &[f64]) -> Vec<u8> {
    let max = data.iter().fold(f64::MIN, |a, &b| a.max(b));
    let min = data.iter().fold(f64::MAX, |a, &b| a.min(b));
    let range = max - min;
    let color_map = colorgrad::preset::turbo(); // You can choose a different color map here
    
    data.iter().flat_map(|&x| {
        let normalized = ((x - min) / range) as f32;
        let color = color_map.at(normalized);
        
        // Color values are in the range [0.0, 1.0], so scale them to [0, 255] and convert to u8
        let [r, g, b,a] = color.to_rgba8();
        vec![r,g,b] // Return RGB values
    }).collect()
}

fn add_shadow(data: &[f64],ncols:usize,nrows:usize,colors:&[u8])->Vec<u8>{
    let hillshadow = shadowing(data, nrows, ncols, 90.0, 315.0, 1.0, 1.0);
    let mut combined = Vec::with_capacity(colors.len());
    
    for i in 0..ncols*nrows {
        let r = colors[i * 3] as f64;
        let g = colors[i * 3 + 1] as f64;
        let b = colors[i * 3 + 2] as f64;
        
        // Adjust color with hillshade (using a simple multiplication)
        let hs = hillshadow[i];
        let adjusted_r = (r * hs).round().clamp(0.0, 255.0) as u8;
        let adjusted_g = (g * hs).round().clamp(0.0, 255.0) as u8;
        let adjusted_b = (b * hs).round().clamp(0.0, 255.0) as u8;
        
        combined.push(adjusted_r);
        combined.push(adjusted_g);
        combined.push(adjusted_b);
    }
    
    combined
}

fn shadowing(data: &[f64],nrows:usize,ncols:usize,altitude: f64, azimuth: f64,cellsize:f64,z_factor:f64) -> Vec<f64>{
    let mut hillshade = vec![0.0; nrows * ncols];
    let azimuth_rad = azimuth.to_radians();
    let altitude_rad = altitude.to_radians();
    
    let zenith_rad = (90.0 - altitude).to_radians();
    let cos_zenith = zenith_rad.cos();
    let sin_zenith = zenith_rad.sin();
    
    for y in 1..nrows-1 {
        for x in 1..ncols-1 {
            let idx = y * ncols + x;
            
            // Calculate slope and aspect using the 3x3 window
            let a = data[idx - ncols - 1];
            let b = data[idx - ncols];
            let c = data[idx - ncols + 1];
            let d = data[idx - 1];
            let f = data[idx + 1];
            let g = data[idx + ncols - 1];
            let h = data[idx + ncols];
            let i = data[idx + ncols + 1];
            
            let dz_dx = ((c + 2.0 * f + i) - (a + 2.0 * d + g)) / (8.0 * cellsize);
            let dz_dy = ((g + 2.0 * h + i) - (a + 2.0 * b + c)) / (8.0 * cellsize);
            
            let slope_rad = (dz_dx.powi(2) + dz_dy.powi(2)).sqrt().atan();
            let aspect_rad = if dz_dx != 0.0 {
                let aspect = dz_dy.atan2(-dz_dx);
                if aspect < 0.0 { aspect + 2.0 * std::f64::consts::PI } else { aspect }
            } else {
                if dz_dy > 0.0 { std::f64::consts::PI / 2.0 }
                else if dz_dy < 0.0 { 3.0 * std::f64::consts::PI / 2.0 }
                else { 0.0 }
            };
            
            // Calculate hillshade value
            let hillshade_value = (cos_zenith * slope_rad.cos() + 
                                  sin_zenith * slope_rad.sin() * 
                                  (azimuth_rad - aspect_rad).cos())
                .max(0.0);
            
            hillshade[idx] = hillshade_value;
        }
    }
    
    // Handle edge cases by copying neighboring values
    for y in 0..nrows {
        for x in 0..ncols {
            if y == 0 || y == nrows-1 || x == 0 || x == ncols-1 {
                let ny = if y == 0 { 1 } else if y == nrows-1 { nrows-2 } else { y };
                let nx = if x == 0 { 1 } else if x == ncols-1 { ncols-2 } else { x };
                hillshade[y * ncols + x] = hillshade[ny * ncols + nx];
            }
        }
    }
    
    hillshade
    }
fn main() {
    let file_path = "0925_6225/LITTO3D_FRA_0929_6224_20150529_LAMB93_RGF93_IGN69/MNT1m/LITTO3D_FRA_0929_6224_MNT_20150128_LAMB93_RGF93_IGN69.asc";
    let file_data =  read_file(file_path);
    let (ncols, nrows, data_vec) = get_data(file_data);
    plot_data(nrows, ncols, data_vec);
}
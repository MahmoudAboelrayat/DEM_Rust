use std::fs::File;
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
    let colors = data_to_color(data);
    let img = RgbImage::from_vec(ncols as u32, nrows as u32, colors).unwrap();
    img.save("output.png").unwrap();
    println!("Image saved as output.png");
}

fn data_to_color(data: Vec<f64>) -> Vec<u8> {
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
fn main() {
    let file_path = "0925_6225/LITTO3D_FRA_0929_6224_20150529_LAMB93_RGF93_IGN69/MNT1m/LITTO3D_FRA_0929_6224_MNT_20150128_LAMB93_RGF93_IGN69.asc";
    let file_data =  read_file(file_path);
    let (ncols, nrows, data_vec) = get_data(file_data);
    plot_data(nrows, ncols, data_vec);
}
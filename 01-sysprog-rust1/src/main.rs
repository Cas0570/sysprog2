use show_image::{create_window, ImageInfo, ImageView};
use std::io::Read;

mod bit_reader;

#[show_image::main]
fn main() {
    // Parse the command line arguments
    let mut args = std::env::args();
    args.next(); // The name of the application
    if args.len() != 1 {
        eprintln!("Usage: viewer <filename>");
        std::process::exit(1);
    }
    let filename = args.next().unwrap();

    // Read the file
    let data = get_file_as_byte_vec(&filename);
    let data_len = data.len();

    // Decode the image
    let start_time = std::time::Instant::now();
    let (width, height, pixels) = decode_image(data);

    // Brag about accomplishments
    println!(
        "Decompressed {} kb of img42 data into {} kb of pixel data in {}s",
        data_len / 1024,
        pixels.len() / 1024,
        start_time.elapsed().as_millis() as f64 / 1000.0
    );

    // Create a window and display the image
    let window = create_window("image", Default::default()).expect("Couldn't create window");
    let image = ImageView::new(ImageInfo::rgb8(width as u32, height as u32), &pixels);
    window
        .set_image("image-001", image)
        .expect("Couldn't display image");

    // Wait for the window to close
    while let Ok(_) = window.run_function_wait(|_window| std::time::Duration::from_secs(3600)) {}
}

/// Decodes/decompresses `data, and returns a tuple containing the image width,
/// image height and RGB pixel data.
fn decode_image(data: Vec<u8>) -> (usize, usize, Vec<u8>) {
    let mut reader = bit_reader::BitReader::new(data);

    for &expected in b"Img42" {
        let actual_byte = reader.get_number(8, false);
        assert_eq!(actual_byte, i64::from(expected), "Invalid Img42 marker");
    }

    let mut pixels: Vec<u8> = Vec::new();
    let width = reader.get_number(16, false) as usize;
    let height = reader.get_number(16, false) as usize;

    for _ in 0..width * height {
        if reader.get_bit() == 1 {
            pixels.extend_from_within(pixels.len() - 3..);
        } else {
            for _ in 0..3 {
                let value = if reader.get_bit() == 1 {
                    i64::from(pixels[pixels.len() - 3]) + reader.get_number(3, true)
                } else {
                    reader.get_number(8, false)
                };

                pixels.push(u8::try_from(value).expect("Channel value out of range"));
            }
        }
    }

    return (width, height, pixels);
}

/// Reads a file by the name of `filename` and returns a byte vector.
fn get_file_as_byte_vec(filename: &String) -> Vec<u8> {
    let mut f = std::fs::File::open(&filename).expect("File not found");
    let metadata = std::fs::metadata(&filename).expect("Unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("Buffer overflow");

    buffer
}

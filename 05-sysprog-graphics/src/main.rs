const WIDTH: usize = 900;
const HEIGHT: usize = 900;
const X_OFFSET: f32 = WIDTH as f32 / 2.0;
const Y_OFFSET: f32 = HEIGHT as f32 / 2.0;

struct Image {
    width: usize,
    height: usize,
    buffer: Vec<u32>,
}

impl Image {
    /// Load a PNG-file as an `Image`.
    fn load(filename: &str) -> Image {
        let image =
            lodepng::decode32_file(&filename).expect(&format!("Couldn't load {}", &filename));
        Image {
            width: image.width,
            height: image.height,
            buffer: image
                .buffer
                .iter()
                .map(|color| {
                    ((color.a as u32) << 24)
                    | ((color.r as u32) << 16)
                    | ((color.g as u32) << 8)
                    | (color.b as u32)
                })
                .collect(),
        }
    }

    fn get_pixel(&self, x: i32, y: i32) -> u32 {
        let x = (x.rem_euclid(self.width as i32)) as usize;
        let y = (y.rem_euclid(self.height as i32)) as usize;
        self.buffer[y * self.width + x]
    }

    fn get_rotated_pixel(&self, x: usize, y: usize, sin: f32, cos: f32) -> u32 {
        // shift, rotate, then look up the pixel in self (not image1)
        let x_shifted = x as f32 - X_OFFSET;
        let y_shifted = y as f32 - Y_OFFSET;
        let x_rotated = x_shifted * cos + y_shifted * sin;
        let y_rotated = x_shifted * sin - y_shifted * cos;
        self.get_pixel(x_rotated as i32, y_rotated as i32)
    }

    fn draw(&self, buffer: &mut [u32], effect: fn(u32) -> u32) {
        for y in 0..self.height.min(HEIGHT){
            for x in 0..self.width.min(WIDTH) {
                buffer[y * WIDTH + x] = effect(self.buffer[y * self.width + x]);
            }
        }
    }
}

fn brightness(color: u32, factor: f32) -> u32 {
    let r = (((color >> 16) & 0xFF) as f32 * factor) as u32;
    let g = (((color >> 8) & 0xFF) as f32 * factor) as u32;
    let b = ((color & 0xFF) as f32 * factor) as u32;
    (r << 16) | (g << 8) | b
}

fn main() {
    // Load the two images
    let image1 = Image::load("mainframe1.png");
    let image2 = Image::load("mainframe2.png");

    // Create a window
    let mut window = minifb::Window::new(
        "Rotating images",
        WIDTH,
        HEIGHT,
        minifb::WindowOptions {
            resize: false,
            topmost: true,
            scale: minifb::Scale::X1,
            ..minifb::WindowOptions::default()
        },
    )
    .expect("Unable to create window");

    // The output buffer that we'll render the output image to
    let mut buffer = vec![0u32; WIDTH * HEIGHT];

    // The current rotation in radians (0..2*Pi)
    let mut rotation = 0.0;

    // Whether we are just drawing a single image (0), rotating a single image (1),
    // or rotating two superimposed images in different directions (2).
    let mut mode = 0;

    // Variables to keep track of frames per second
    let mut fps_time = std::time::Instant::now();
    let mut fps_count = 0;

    // The main loop
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        // Rotate a bit
        rotation += 0.01;

        // Change the mode
        if window.is_key_pressed(minifb::Key::Space, minifb::KeyRepeat::No) {
            mode = (mode + 1) % 6;
        }

        // Draw!
        // To prevent code duplication, you may want to move your code from
        // your `modes` answers into a function or a method of `Image`.
        if mode == 0 {
            image1.draw(&mut buffer, |c| c);
        } else if mode == 1 {
            image1.draw(&mut buffer, |c| brightness(c, 0.25));
        } else if mode == 2 {
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let p1 = image1.get_pixel(x as i32, y as i32);
                    let p2 = image2.get_pixel(x as i32, y as i32);
                    buffer[y * WIDTH + x] = brightness(p1, 0.5) + brightness(p2, 0.5);
                }
            }  
        } else if mode == 3 {
            let angle: f32 = rotation;
            let (sin, cos) = angle.sin_cos();
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    buffer[y * WIDTH + x] = image1.get_rotated_pixel(x, y, sin, cos);
                }
            }
        } else if mode == 4 {
            let angle: f32 = rotation;
            let (sin, cos) = angle.sin_cos();
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let p1 = image1.get_rotated_pixel(x, y, sin, cos);
                    let p2 = image2.get_rotated_pixel(x, y, -sin, cos);
                    buffer[y * WIDTH + x] = brightness(p1, 0.5) + brightness(p2, 0.5);
                }
            }
        } else {
            const SHIFT: i32 = 16;
            let scale = (1 << SHIFT) as f32;
            let angle: f32 = rotation;
            let (sin, cos) = angle.sin_cos();
            let cos_fp = (cos * scale).round() as i32;
            let sin_fp = (sin * scale).round() as i32;
            let (w1, h1) = ((image1.width as i32) << SHIFT, (image1.height as i32) << SHIFT);
            let (w2, h2) = ((image2.width as i32) << SHIFT, (image2.height as i32) << SHIFT);
            for y in 0..HEIGHT {
                let x_shifted = -X_OFFSET;
                let y_shifted = y as f32 - Y_OFFSET;
                let mut x1 = (((x_shifted * cos + y_shifted * sin) * scale) as i32).rem_euclid(w1);
                let mut y1 = (((x_shifted * sin - y_shifted * cos) * scale) as i32).rem_euclid(h1);
                let mut x2 = (((x_shifted * cos - y_shifted * sin) * scale) as i32).rem_euclid(w2);
                let mut y2 = (((x_shifted * -sin - y_shifted * cos) * scale) as i32).rem_euclid(h2);

                for x in 0..WIDTH {
                    let p1 = image1.buffer[(y1 >> SHIFT) as usize * image1.width + (x1 >> SHIFT) as usize];
                    let p2 = image2.buffer[(y2 >> SHIFT) as usize * image2.width + (x2 >> SHIFT) as usize];
                    buffer[y * WIDTH + x] = ((p1 >> 1) & 0x7F7F7F) + ((p2 >> 1) & 0x7F7F7F);

                    x1 += cos_fp;
                    if x1 >= w1 { x1 -= w1 } else if x1 < 0 { x1 += w1 }
                    y1 += sin_fp;
                    if y1 >= h1 { y1 -= h1 } else if y1 < 0 { y1 += h1 }
                    x2 += cos_fp;
                    if x2 >= w2 { x2 -= w2 } else if x2 < 0 { x2 += w2 }
                    y2 -= sin_fp;
                    if y2 >= h2 { y2 -= h2 } else if y2 < 0 { y2 += h2 }
                }
            }
        }

        // Show the buffer in the window
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();

        // Display the FPS rate 3 times per second
        let now = std::time::Instant::now();
        let duration = (now - fps_time).as_secs_f32();
        fps_count += 1;
        if duration >= 0.33 {
            println!("{} fps", (fps_count as f32 / duration).round() as u32);
            fps_time = now;
            fps_count = 0;
        }
    }
}

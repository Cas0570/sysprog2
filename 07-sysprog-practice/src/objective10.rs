use rand::Rng;
use minifb::Key;
use std::thread;


/// Output image width
const WIDTH: usize = 960;
/// Output image height
const HEIGHT: usize = 480;

const THREAD_COUNT: usize = 8;
const ROWS_PER_STRIP: usize = HEIGHT / THREAD_COUNT;
const PIXELS_PER_STRIP: usize = WIDTH * ROWS_PER_STRIP;


/// Structure used for decoded PNG files as well as for the target buffer
/// that we'll draw on.
struct Image {
    width: u32,
    height: u32,
    /// Each value in the `buffer` vector represents one pixel, ordered from top to bottom, from left to right.
    /// So given a 10x10 Image, element 26 is at the third line (y=2) and the seventh column (x=6).
    /// 
    /// For each pixel, bits 0 through 7 (the least significant bits) represent the color blue,
    /// 8 through 15 represent green and 16 through 23 represent red. Bits 24 through 31 represent the
    /// opacity (or alpha value), where 255 means fully opaque (you cannot see through it), 0 means
    /// invisible, and 128 means 50% transparency. The opacity value is ignore when outputting to the
    /// screen using `minifb`.
    buffer: Vec<u32>,
}

impl Image {
    /// Load the PNG-file with the given `filename` and create an `Image` object out of it.
    fn load(filename: &str) -> Image {
        let bitmap = lodepng::decode32_file(&filename).expect(&format!("Couldn't load {}", &filename));
        Image {
            width: bitmap.width as u32,
            height: bitmap.height as u32,
            buffer: bitmap.buffer.iter().map(|color| { // convert to u32
                (color.a as u32) << 24 |
                (color.r as u32) << 16 |
                (color.g as u32) << 8 |
                (color.b as u32) << 0
            }).collect(),
        }
    }

    /// Draw the source Image (`self`) onto the `target` Image, with the top left of the
    /// source at `(offset_x, offset_y)` relative to the the top left of the `target` 
    /// Image. Parts of the source image that will fall outside the target image are ignored.
    fn draw(&self, target: &mut RenderTarget<'_>, offset_x: i32, offset_y: i32) {
        // Use wider signed coordinates so clipping also handles extreme offsets.
        let offset_x = i64::from(offset_x);
        let offset_y = i64::from(offset_y) - i64::from(target.y_offset);
        let start_x = offset_x.max(0);
        let end_x = (offset_x + i64::from(self.width)).min(i64::from(target.width));
        let start_y = offset_y.max(0);
        let end_y = (offset_y + i64::from(self.height)).min(i64::from(target.height));

        if start_x >= end_x || start_y >= end_y {
            return;
        }

        let source_width = self.width as usize;
        let target_width = target.width as usize;
        let draw_width = (end_x - start_x) as usize;
        let draw_height = (end_y - start_y) as usize;
        let mut source_start =
            (start_y - offset_y) as usize * source_width + (start_x - offset_x) as usize;
        let mut target_start = start_y as usize * target_width + start_x as usize;

        for _ in 0..draw_height {
            // Slice once per row, then visit matching pixels without index arithmetic.
            let source_row = &self.buffer[source_start..source_start + draw_width];
            let target_row = &mut target.buffer[target_start..target_start + draw_width];

            for (&pixel, target_pixel) in source_row.iter().zip(target_row.iter_mut()) {
                let alpha = pixel >> 24;

                // Approximate /255 with /256 (>> 8). Weights sum to 256,
                // avoiding the darkening caused by simply changing the divisor.
                // Alpha 0 and 255 produce exact target and source RGB values.
                let source_weight = alpha + u32::from(alpha != 0);
                let target_weight = 256 - source_weight;
                let previous = *target_pixel;
                // Red and blue have an empty byte between them, so their
                // weighted sums can be calculated together without overlap.
                let red_blue = (((pixel & 0x00ff_00ff) * source_weight
                    + (previous & 0x00ff_00ff) * target_weight) >> 8) & 0x00ff_00ff;
                let green = (((pixel & 0x0000_ff00) * source_weight
                    + (previous & 0x0000_ff00) * target_weight) >> 8) & 0x0000_ff00;

                *target_pixel = 0xff00_0000 | red_blue | green;
            }

            source_start += source_width;
            target_start += target_width;
        }
    }
}


struct RenderTarget<'a> {
    width: u32,
    height: u32,
    y_offset: i32,
    buffer: &'a mut [u32],
}


trait Drawable: Sync {
    fn update(&mut self, keys: &[Key]) -> bool;
    fn draw(&self, target: &mut RenderTarget<'_>);
}


struct FpsIndicator {
    font: Vec<u8>,
    interval_start: std::time::Instant,
    frame_count: u32,
    fps: u32,
}

impl FpsIndicator {
    fn new() -> Self {
        Self {
            font: std::fs::read("font14.bin")
                .expect("Couldn't load font14.bin"),
            interval_start: std::time::Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }
}

impl Drawable for FpsIndicator {
    fn update(&mut self, _keys: &[Key]) -> bool {
        self.frame_count += 1;

        let now = std::time::Instant::now();
        let elapsed = (now - self.interval_start).as_secs_f32();

        if elapsed >= 1.0 {
            self.fps = self.frame_count;
            self.frame_count = 0;
            self.interval_start = now;
        }

        true
    }

    fn draw(&self, target: &mut RenderTarget<'_>) {
        let strip_start = target.y_offset;
        let strip_end = strip_start + target.height as i32;

        if strip_start >= 24 || strip_end <= 10 {
            return;
        }

        let text = self.fps.to_string();

        for (digit_index, character) in text.bytes().enumerate() {
            let glyph_start = character as usize * 14;

            for row in 0..14 {
                let row_bits = self.font[glyph_start + row];

                for column in 0..8 {
                    let mask = 0b1000_0000u8 >> column;

                    if (row_bits & mask) != 0 {
                        let x = 10 + digit_index * 8 + column;
                        let y = (10 + row) as i32 - target.y_offset;

                        if x < target.width as usize 
                            && y >= 0 
                            && y < target.height as i32 
                        {
                            let index = y as usize * target.width as usize + x;
                            target.buffer[index] = 0xffff_ffff;
                        }
                    }
                }
            }
        }
    }
}


struct BackgroundLayer {
    image: Image,
    x: f32,
    speed: f32,
}

impl BackgroundLayer {
    fn new(filename: &str, speed: f32) -> Self {
        Self {
            image: Image::load(filename),
            x: 0.0,
            speed,
        }
    }
}

impl Drawable for BackgroundLayer {
    fn update(&mut self, _keys: &[Key]) -> bool {
        self.x -= self.speed;

        let width = self.image.width as f32;

        while self.x <= -width {
            self.x += width;
        }

        true
    }

    fn draw(&self, target: &mut RenderTarget<'_>) {
        let mut draw_x = self.x as i32;

        while draw_x < target.width as i32 {
            self.image.draw(target, draw_x, 0);
            draw_x += self.image.width as i32;
        }
    }
}


struct FireflyGroup<'a> {
    image: &'a Image,
    x: f32,
    y: f32,
    x_speed: f32,
    y_speed: f32,
    offsets: Vec<(f32, f32)>,
}

impl<'a> FireflyGroup<'a> {
    fn new(image: &'a Image) -> Self {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(1..=12);
        let mut offsets = Vec::new();

        for _ in 0..count {
            offsets.push((
                rng.gen_range(-30.0..30.0),
                rng.gen_range(-30.0..30.0),
            ))
        }

        Self {
            image,
            x: WIDTH as f32,
            y: rng.gen_range(0.0..HEIGHT as f32),
            x_speed: -8.0,
            y_speed: rng.gen_range(-1.0..1.0),
            offsets,
        }
    }
}

impl<'a> Drawable for FireflyGroup<'a> {
    fn update(&mut self, _keys: &[Key]) -> bool {
        let mut rng = rand::thread_rng();

        self.x_speed += rng.gen_range(-0.1..0.1);
        self.y_speed += rng.gen_range(-0.1..0.1);

        self.x_speed = self.x_speed.min(-1.0);

        self.x += self.x_speed;
        self.y += self.y_speed;

        for (offset_x, offset_y) in &mut self.offsets {
            *offset_x += rng.gen_range(-0.5..0.5);
            *offset_y += rng.gen_range(-0.5..0.5);
        }

        self.offsets.iter().any(|&(offset_x, _)| {
            let right_edge = self.x + offset_x + self.image.width as f32;
            if right_edge > 0.0 { true } else { false }
        })
    }

    fn draw(&self, target: &mut RenderTarget<'_>) {
        for &(offset_x, offset_y) in &self.offsets {
            let x = self.x + offset_x;
            let y = self.y + offset_y;

            self.image.draw(target, x as i32, y as i32);
        }
    }
}


struct Dragon {
    frames: Vec<Image>,
    frame_index: usize,
    y: f32,
    y_speed: f32,
}

impl Dragon {
    fn new() -> Self {
        let mut frames = Vec::new();

        for number in 1..=20 {
            let filename = format!("dragon-{}.png", number);
            frames.push(Image::load(&filename));
        }

        Self {
            frames,
            frame_index: 0,
            y: 100.0,
            y_speed: 0.0,
        }
    }
}

impl Drawable for Dragon {
    fn update(&mut self, keys: &[Key]) -> bool {
        self.frame_index = (self.frame_index + 1) % self.frames.len();

        if keys.contains(&Key::Space) {
            self.y_speed -= 0.2;
        } else {
            self.y_speed += 0.2;
        }
        self.y += self.y_speed;

        true
    }

    fn draw(&self, target: &mut RenderTarget<'_>) {
        let frame = &self.frames[self.frame_index];
        frame.draw(target, 100, self.y as i32)
    }
}



/// The main game loop. You should not need to change anything here (but you can!),
/// except for (un)commenting code at the *TODO* item.
fn main() {
    // Load the firefly Image here and pass it to FireflyGroup instances, to prevent
    // each instance from having to load the Image for itself.
    let firefly_image = Image::load("firefly.png");

    // Create the vector containing all game objects that should be drawn.
    // They are drawn in order, so later object overdraw earlier objects.
    let mut drawables: Vec<Box<dyn Drawable>> = Vec::new();

    // Spawn background layers
    for num in 1..8 {
        drawables.push(Box::new(BackgroundLayer::new(
            &format!("bg{}.png", num),
            (num - 1) as f32
        )));
    }

    // Spawn the dragon
    drawables.insert(3, Box::new(Dragon::new()));

    // Spawn the FPS indicator
    drawables.push(Box::new(FpsIndicator::new()));
    
    // Initialize a random number generator
    let mut rng = rand::thread_rng();

    // Create the target Image that drawables will draw to.
    let mut target = Image {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        buffer: vec![0; WIDTH * HEIGHT],
    };

    // Create a window
    let mut window = minifb::Window::new(
        "Dragon vs Fireflies",
        WIDTH as usize,
        HEIGHT as usize,
        minifb::WindowOptions {
            resize: false,
            topmost: true,
            scale: minifb::Scale::X1,
            ..minifb::WindowOptions::default()
        },
    ).expect("Unable to create window");

    // Limit to max ~30 fps update rate
    let mut limiter = true;

    // The main loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Spawn new firefly groups every so often
        if rng.gen_range(0..10) == 0 {
            drawables.push(Box::new(FireflyGroup::new(&firefly_image)));
        }

        // Update all drawables
        let keys = window.get_keys();
        let mut new_drawables: Vec<Box<dyn Drawable>> = Vec::new();
        for mut drawable in drawables {
            if drawable.update(&keys) {
                new_drawables.push(drawable);
            }
        }
        drawables = new_drawables;

        thread::scope(|scope| {
            for (strip_index, buffer) in target.buffer
                .chunks_mut(PIXELS_PER_STRIP)
                .enumerate()
            {
                let mut render_target = RenderTarget {
                    width: WIDTH as u32,
                    height: ROWS_PER_STRIP as u32,
                    y_offset: strip_index as i32 * ROWS_PER_STRIP as i32,
                    buffer,
                };

                let shared_drawables = &drawables;

                scope.spawn(move || {
                    for drawable in shared_drawables {
                        drawable.draw(&mut render_target);
                    }
                });
            }
        });

        // Enable/disable the FPS limiter
        if window.is_key_pressed(Key::L, minifb::KeyRepeat::No) {
            limiter = !limiter;
        }
        if limiter {
            window.limit_update_rate(Some(std::time::Duration::from_micros(33200)));
        } else {
            window.limit_update_rate(None);
        }

        // Show the output in the window
        window
           .update_with_buffer(&target.buffer, target.width as usize, target.height as usize)
           .unwrap();
    }
}

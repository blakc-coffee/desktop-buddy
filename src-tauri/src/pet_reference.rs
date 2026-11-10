// Render the approved artwork itself. Never substitute a hand-drawn approximation.
// Source PNGs are immutable; only the connected presentation background is omitted.
struct ReferencePet {
    width: i32,
    height: i32,
    pixels: Vec<u8>,
    bounds: [i32; 4],
    eyes: [i32; 4],
}

fn reference_spec(pet: Appearance) -> Option<(usize, &'static [u8], [i32; 4])> {
    use Appearance::*;
    Some(match pet {
        Calico => (
            0,
            include_bytes!("../../assets/pet-reference/calico.png"),
            [590, 566, 720, 566],
        ),
        Siamese => (
            1,
            include_bytes!("../../assets/pet-reference/siamese.png"),
            [590, 586, 690, 586],
        ),
        BlueCat => (
            2,
            include_bytes!("../../assets/pet-reference/blue-cat.png"),
            [586, 570, 688, 570],
        ),
        Dachshund => (
            3,
            include_bytes!("../../assets/pet-reference/dachshund.png"),
            [494, 585, 599, 585],
        ),
        Samoyed => (
            4,
            include_bytes!("../../assets/pet-reference/samoyed.png"),
            [604, 590, 698, 590],
        ),
        Collie => (
            5,
            include_bytes!("../../assets/pet-reference/collie.png"),
            [588, 594, 701, 594],
        ),
        RedPanda => (
            6,
            include_bytes!("../../assets/pet-reference/red-panda.png"),
            [607, 580, 697, 580],
        ),
        Otter => (
            7,
            include_bytes!("../../assets/pet-reference/otter.png"),
            [489, 642, 589, 642],
        ),
        Capybara => (
            8,
            include_bytes!("../../assets/pet-reference/capybara.png"),
            [492, 580, 625, 580],
        ),
        _ => return None,
    })
}

impl ReferencePet {
    fn decode(bytes: &[u8], eyes: [i32; 4]) -> Self {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().expect("embedded approved pet PNG");
        let mut source = vec![0; reader.output_buffer_size().expect("bounded embedded PNG")];
        let info = reader
            .next_frame(&mut source)
            .expect("valid embedded pet pixels");
        assert!(matches!(
            info.color_type,
            png::ColorType::Rgb | png::ColorType::Rgba
        ));
        let (width, height) = (info.width as usize, info.height as usize);
        let channels = if info.color_type == png::ColorType::Rgba {
            4
        } else {
            3
        };
        // Flood from the image edge, not a global color replacement: enclosed dark
        // pupils, mouths and coat details remain byte-for-byte source colors.
        let mut background = vec![false; width * height];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(0usize);
        while let Some(i) = queue.pop_front() {
            if background[i] {
                continue;
            }
            let p = &source[i * channels..i * channels + 3];
            if p.iter().any(|v| *v > 50) {
                continue;
            }
            background[i] = true;
            let (x, y) = (i % width, i / width);
            if x > 0 {
                queue.push_back(i - 1);
            }
            if x + 1 < width {
                queue.push_back(i + 1);
            }
            if y > 0 {
                queue.push_back(i - width);
            }
            if y + 1 < height {
                queue.push_back(i + width);
            }
        }
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for (i, bg) in background.iter().enumerate() {
            if !bg {
                left = left.min(i % width);
                right = right.max(i % width + 1);
                top = top.min(i / width);
                bottom = bottom.max(i / width + 1);
            }
        }
        assert!(
            right > left && bottom > top && right - left < width / 2 && bottom - top < height / 2,
            "approved pet must be isolated from its presentation background"
        );
        let (w, h) = (right - left, bottom - top);
        let mut pixels = vec![0; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (top + y) * width + left + x;
                if !background[i] {
                    let p = &source[i * channels..i * channels + 3];
                    pixels[(y * w + x) * 4..(y * w + x + 1) * 4]
                        .copy_from_slice(&[p[2], p[1], p[0], 255]);
                }
            }
        }
        Self {
            width: w as i32,
            height: h as i32,
            pixels,
            bounds: [left as i32, top as i32, right as i32, bottom as i32],
            eyes,
        }
    }
    fn destination(&self) -> [i32; 4] {
        let scale = (920.0 / self.width as f32).min(880.0 / self.height as f32);
        let (w, h) = (
            (self.width as f32 * scale).round() as i32,
            (self.height as f32 * scale).round() as i32,
        );
        [660 - w / 2, 1040 - h, w, h]
    }
    fn eye_anchors(&self) -> (i32, i32, i32, i32) {
        let [x, y, w, h] = self.destination();
        let px = |p| x + (p - self.bounds[0]) * w / self.width;
        let py = |p| y + (p - self.bounds[1]) * h / self.height;
        (
            px(self.eyes[0]),
            py(self.eyes[1]),
            px(self.eyes[2]),
            py(self.eyes[3]),
        )
    }
}

fn reference_pet(pet: Appearance) -> Option<&'static ReferencePet> {
    // Decode each chosen pet only once, retaining its cropped pixels (not the
    // 1254px presentation canvas). No disk/network access or per-frame decode.
    static CACHE: [std::sync::OnceLock<ReferencePet>; 9] =
        [const { std::sync::OnceLock::new() }; 9];
    let (index, bytes, eyes) = reference_spec(pet)?;
    Some(CACHE[index].get_or_init(|| ReferencePet::decode(bytes, eyes)))
}
fn reference_eyes(pet: Appearance) -> Option<(i32, i32, i32, i32)> {
    Some(reference_pet(pet)?.eye_anchors())
}
unsafe fn draw_reference_pet(canvas: &Canvas, pet: Appearance) -> bool {
    let Some(sprite) = reference_pet(pet) else {
        return false;
    };
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 40,
            biWidth: sprite.width,
            biHeight: -sprite.height,
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    let source_dc = CreateCompatibleDC(Some(canvas.dc));
    let mut bits = std::ptr::null_mut();
    let Ok(bitmap) = CreateDIBSection(Some(source_dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
    else {
        let _ = DeleteDC(source_dc);
        return false;
    };
    std::ptr::copy_nonoverlapping(sprite.pixels.as_ptr(), bits.cast(), sprite.pixels.len());
    let old = SelectObject(source_dc, bitmap.into());
    let [x, y, w, h] = sprite.destination();
    // The existing world transform supplies context-specific lean, bounce and
    // scaling. TransparentBlt preserves the approved pixels and dark details.
    let success = TransparentBlt(
        canvas.dc,
        canvas.scaled(x),
        canvas.scaled(y),
        canvas.scaled(w),
        canvas.scaled(h),
        source_dc,
        0,
        0,
        sprite.width,
        sprite.height,
        0,
    )
    .as_bool();
    SelectObject(source_dc, old);
    let _ = DeleteObject(bitmap.into());
    let _ = DeleteDC(source_dc);
    success
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn approved_pixels_are_isolated_bounded_and_eye_anchors_fit() {
        let mut bytes = 0;
        for pet in crate::pet::APPEARANCES {
            let Some(sprite) = reference_pet(pet) else {
                continue;
            };
            assert!(sprite.width > 200 && sprite.height > 150);
            assert!(sprite.pixels.chunks_exact(4).any(|p| p[3] == 0));
            assert!(sprite
                .pixels
                .chunks_exact(4)
                .any(|p| p[3] == 255 && p[0] < 55 && p[1] < 55 && p[2] < 55));
            let (x, y, r, ry) = sprite.eye_anchors();
            let [left, top, w, h] = sprite.destination();
            assert!(x >= left && r <= left + w && y >= top && ry <= top + h);
            bytes += sprite.pixels.len();
        }
        assert!(
            bytes < 8 * 1024 * 1024,
            "all nine decoded crops stay below 8 MiB"
        );
    }
}

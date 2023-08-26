use rand::Rng;
use std::cmp;
use std::ptr;
use std::thread;
use std::time;
use x11::xlib;

struct Box {
    section_start: u16,
    section_end: u16,
    side: u16, //0 top, 1 right, 2 bottom, 3 left
    mean_x_min: u16,
    mean_x_max: u16,
    mean_y_min: u16,
    mean_y_max: u16,
    r: u16,
    g: u16,
    b: u16,
}

impl Box {
    fn set_color(&mut self, r: u16, g: u16, b: u16) {
        self.r = r;
        self.b = g;
        self.g = b;
    }
}

struct Led {
    box_idx: u16,
    relative_box_position: u16,
    r: u16,
    g: u16,
    b: u16,
}

pub struct Screen {
    display: *mut xlib::Display,
    window: xlib::Window,
}

impl Screen {
    /// Tries to open the X11 display, then returns a handle to the default screen.
    ///
    /// Returns `None` if the display could not be opened.
    pub fn open() -> Option<Screen> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen = xlib::XDefaultScreenOfDisplay(display);
            let root = xlib::XRootWindowOfScreen(screen);
            Some(Screen {
                display,
                window: root,
            })
        }
    }

    pub fn capture_ximage(&self, w: u32, h: u32, x: i32, y: i32) -> *mut xlib::XImage {
        let img =
            unsafe { xlib::XGetImage(self.display, self.window, x, y, w, h, !1, xlib::ZPixmap) };
        return img;
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        unsafe {
            xlib::XCloseDisplay(self.display);
        }
    }
}

fn new_box(
    idx: u16,
    width: u16,
    side: u16,
    mean_radius: u16,
    mean_depth: u16,
    res_in_dir: u16,
    res_ortho: u16,
) -> Box {
    let section_center: u16 = (idx * width) + width / 2;
    let in_dir_mean_min = (u16::MIN + section_center)
        .checked_sub(mean_radius)
        .unwrap_or(0);
    let in_dir_mean_max = cmp::min(section_center + mean_radius, res_in_dir);

    Box {
        section_start: idx * width,
        section_end: (idx + 1) * width - 1,
        side,
        mean_x_min: match side {
            0 | 2 => in_dir_mean_min,
            1 => res_ortho - mean_depth - 1,
            3 => 0,
            _ => 0,
        },
        mean_x_max: match side {
            0 | 2 => in_dir_mean_max,
            1 => res_ortho - 1,
            3 => mean_depth - 1,
            _ => 0,
        },
        mean_y_min: match side {
            0 => 0,
            2 => res_ortho - mean_depth - 1,
            1 | 3 => in_dir_mean_min,
            _ => 0,
        },
        mean_y_max: match side {
            0 => mean_depth - 1,
            2 => res_ortho - 1,
            1 | 3 => in_dir_mean_max,
            _ => 0,
        },
        r: 0,
        g: 0,
        b: 0,
    }
}

fn new_led(boxes: &Vec<Box>, led_pos: u16, side: u16) -> Led {
    let (idx, bx) = boxes
        .iter()
        .enumerate()
        .find(|(_, b)| b.section_start <= led_pos && b.section_end >= led_pos && b.side == side)
        .unwrap();

    Led {
        box_idx: idx as u16,
        relative_box_position: (led_pos - bx.section_start) * 100
            / (bx.section_end - bx.section_start),
        r: 0,
        g: 0,
        b: 0,
    }
}

fn get_boxes(
    res_x: u16,
    res_y: u16,
    x_box_cnt: u16,
    y_box_cnt: u16,
    mean_depth: u16,
    mean_radius: u16,
) -> Vec<Box> {
    let box_cnt: u16 = 2 * (x_box_cnt + y_box_cnt);
    let mut boxes = Vec::with_capacity(usize::from(box_cnt));
    let x_width: u16;
    let y_width: u16;
    x_width = res_x / x_box_cnt;
    y_width = res_y / y_box_cnt;

    //top boxes
    for idx in 0..x_box_cnt {
        boxes.push(new_box(
            idx,
            x_width,
            0,
            mean_radius,
            mean_depth,
            res_x,
            res_y,
        ));
    }

    //right boxes
    for idx in 0..y_box_cnt {
        boxes.push(new_box(
            idx,
            y_width,
            1,
            mean_radius,
            mean_depth,
            res_y,
            res_x,
        ));
    }

    //bottom boxes
    for idx in (0..x_box_cnt).rev() {
        boxes.push(new_box(
            idx,
            x_width,
            2,
            mean_radius,
            mean_depth,
            res_x,
            res_y,
        ));
    }

    //left boxes
    for idx in (0..y_box_cnt).rev() {
        boxes.push(new_box(
            idx,
            y_width,
            3,
            mean_radius,
            mean_depth,
            res_y,
            res_x,
        ));
    }

    return boxes;
}

fn get_leds(
    boxes: &Vec<Box>,
    x_led_count: u16,
    y_led_count: u16,
    res_x: u16,
    res_y: u16,
) -> Vec<Led> {
    let mut leds = Vec::with_capacity(usize::from(2 * (x_led_count + y_led_count)));

    //top leds
    for idx in 0..x_led_count {
        let led_pos: u16 = (res_x / x_led_count) * idx;
        leds.push(new_led(&boxes, led_pos, 0));
    }

    //right leds
    for idx in 0..y_led_count {
        let led_pos: u16 = (res_y / y_led_count) * idx;
        leds.push(new_led(&boxes, led_pos, 1));
    }

    //bottom leds
    for idx in (0..x_led_count).rev() {
        let led_pos: u16 = (res_x / x_led_count) * idx;
        leds.push(new_led(&boxes, led_pos, 2));
    }

    //left leds
    for idx in (0..y_led_count).rev() {
        let led_pos: u16 = (res_y / y_led_count) * idx;
        leds.push(new_led(&boxes, led_pos, 3));
    }

    return leds;
}

fn color_boxes(boxes: &mut Vec<Box>, screen: &Screen, sample_count: u16) {
    boxes.iter_mut().for_each(|b| {
        let mean_color: Vec<u16> = (0..sample_count)
            .collect::<Vec<u16>>()
            .iter()
            .map(|_| {
                let px = rand::thread_rng().gen_range(b.mean_x_min..b.mean_x_max);
                let py = rand::thread_rng().gen_range(b.mean_y_min..b.mean_y_max);
                let screen_capture = screen.capture_ximage(1, 1, px.into(), py.into());
                let clr = &mut xlib::XColor {
                    pixel: unsafe { xlib::XGetPixel(screen_capture, 0, 0) },
                    red: 0,
                    green: 0,
                    blue: 0,
                    flags: 0,
                    pad: 0,
                };

                unsafe {
                    xlib::XQueryColor(
                        screen.display,
                        xlib::XDefaultColormap(
                            screen.display,
                            xlib::XDefaultScreen(screen.display),
                        ),
                        clr,
                    )
                };
                unsafe { xlib::XDestroyImage(screen_capture as *mut _) };

                return [clr.red / 256, clr.green / 256, clr.blue / 256];
            })
            .map(|c| [c[0] as u16, c[1] as u16, c[2] as u16])
            .fold([0, 0, 0], |[rs, gs, bs], [r, g, b]| {
                [rs + r, gs + g, bs + b]
            })
            .iter()
            .map(|c| c / sample_count)
            .collect();

        b.set_color(mean_color[0], mean_color[1], mean_color[2]);
    });
}

impl Led {
    fn update_color(&mut self, boxes: &Vec<Box>) {
        let boxes_last_idx: u16 = boxes.len() as u16 - 1;
        let current_box = &boxes[usize::from(self.box_idx)];
        let next_box = match current_box.side {
            0 | 1 => &boxes[usize::from(self.box_idx) + 1],
            2 | 3 => &boxes[usize::from(self.box_idx) - 1],
            _ => &boxes[0],
        };
        let previous_box = match current_box.side {
            0 | 1 => match self.box_idx {
                0 => &boxes[usize::from(boxes_last_idx)],
                _ => &boxes[usize::from(self.box_idx) + 1],
            },
            2 | 3 => match self.box_idx {
                a if a == boxes_last_idx => &boxes[0],
                _ => &boxes[usize::from(self.box_idx) - 1],
            },
            _ => &boxes[0],
        };

        let (b1, b2, relative_unit) = match self.relative_box_position.cmp(&50) {
            cmp::Ordering::Less => (previous_box, current_box, self.relative_box_position * 2),
            cmp::Ordering::Greater | cmp::Ordering::Equal => {
                (current_box, next_box, (self.relative_box_position - 50) * 2)
            }
        };

        self.r = (relative_unit * b2.r + (100 - relative_unit) * b1.r) / 100;
        self.g = (relative_unit * b2.g + (100 - relative_unit) * b1.g) / 100;
        self.b = (relative_unit * b2.b + (100 - relative_unit) * b1.b) / 100;
    }
}

fn main() {
    let x_res = 2560;
    let y_res = 1440;
    let x_box_cnt = 9;
    let y_box_cnt = 5;
    let mean_radius = 150;
    let mean_depth = 300;
    let x_led_count = 86;
    let y_led_count = 35;
    let sampling_size = 12;
    let loop_sleep = 100;

    let mut boxes = get_boxes(x_res, y_res, x_box_cnt, y_box_cnt, mean_depth, mean_radius);
    let mut leds = get_leds(&boxes, x_led_count, y_led_count, x_res, y_res);

    let screen = Screen::open().unwrap();

    loop {
        color_boxes(&mut boxes, &screen, sampling_size);
        leds.iter_mut().for_each(|l| l.update_color(&boxes));

        leds.iter()
            .for_each(|b| println!("{} {} {}", b.r, b.g, b.b));

        thread::sleep(time::Duration::from_millis(loop_sleep));
    }
    /*
    boxes.iter()
        .for_each(|b| {println!("{} {} {} {} {} {} {}", b.section_start, b.section_end, b.mean_x_min, b.mean_y_min, b.mean_x_max, b.mean_y_max, b.side)});
    boxes.iter()
        .for_each(|b| {println!("{} {} {}", b.r, b.g, b.b)});
    leds.iter()
        .for_each(|l| {println!("{} {}", l.box_idx, l.relative_box_position)});
    */
}

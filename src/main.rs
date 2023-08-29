use rand::Rng;
use std::time::{Duration, Instant};
use std::cmp;
use std::ptr;
use std::thread;
use x11::xlib;
use std::io::{self, Write};

struct Box {
    section_start: u16,
    section_end: u16,
    side: u16, //0 top, 1 right, 2 bottom, 3 left
    sample_points : Vec<Vec<u16>>,
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
    r: u8,
    g: u8,
    b: u8,
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
        sample_points: Vec::new(),
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

fn set_regular_sampling_points(boxes: &mut Vec<Box>, sampling_size: u16) {
    boxes.iter_mut()
        .for_each(|b| {
            (0..sampling_size)
                .collect::<Vec<u16>>()
                .iter()
                .for_each(|_| {
                    let px = rand::thread_rng().gen_range(b.mean_x_min..b.mean_x_max);
                    let py = rand::thread_rng().gen_range(b.mean_y_min..b.mean_y_max);
                    b.sample_points.push(vec![px, py]);
                })
        })
}

fn get_boxes(
    res_x: u16,
    res_y: u16,
    x_box_cnt: u16,
    y_box_cnt: u16,
    mean_depth: u16,
    mean_radius: u16,
    sampling_size: u16,
    random_sampling: bool
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

    if !random_sampling {
        set_regular_sampling_points(&mut boxes, sampling_size);
    }

    return boxes;
}

fn get_leds(
    boxes: &Vec<Box>,
    x_led_count: u16,
    y_led_count: u16,
    res_x: u16,
    res_y: u16,
    led_groups_pop: u8
) -> Vec<Led> {

    let x_group_count: u16 = x_led_count / (led_groups_pop as u16);
    let y_group_count: u16 = y_led_count / (led_groups_pop as u16);

    let mut leds = Vec::with_capacity(usize::from(2 * (x_group_count + y_group_count)));

    //top leds
    for idx in 0..x_group_count {
        let led_pos: u16 = (res_x  / x_group_count) * idx;
        leds.push(new_led(&boxes, led_pos, 0));
    }

    //right leds
    for idx in 0..y_group_count {
        let led_pos: u16 = (res_y / y_group_count) * idx;
        leds.push(new_led(&boxes, led_pos, 1));
    }

    //bottom leds
    for idx in (0..x_group_count).rev() {
        let led_pos: u16 = (res_x / x_group_count) * idx;
        leds.push(new_led(&boxes, led_pos, 2));
    }

    //left leds
    for idx in (0..y_group_count).rev() {
        let led_pos: u16 = (res_y / y_group_count) * idx;
        leds.push(new_led(&boxes, led_pos, 3));
    }

    return leds;
}

fn color_boxes(boxes: &mut Vec<Box>, screen: &Screen, sample_count: u16, random_sampling: bool) {
    boxes.iter_mut().for_each(|b| {
        let mean_color: Vec<u16> = (0..sample_count)
            .collect::<Vec<u16>>()
            .iter()
            .map(|&idx| {
                let px;
                let py;
                if random_sampling {
                    px = rand::thread_rng().gen_range(b.mean_x_min..b.mean_x_max);
                    py = rand::thread_rng().gen_range(b.mean_y_min..b.mean_y_max);
                } else {
                    px = b.sample_points[usize::from(idx)][0];
                    py = b.sample_points[usize::from(idx)][1];
                }

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
    fn update_color(&mut self, boxes: &Vec<Box>, luminosity: u16) {
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

        self.r = (luminosity * ((relative_unit * b2.r + (100 - relative_unit) * b1.r) / 100) / 100) as u8;
        self.g = (luminosity * ((relative_unit * b2.g + (100 - relative_unit) * b1.g) / 100) / 100) as u8;
        self.b = (luminosity * ((relative_unit * b2.b + (100 - relative_unit) * b1.b) / 100) / 100) as u8;
    }
}

fn color_leds(leds: &mut Vec<Led>, boxes: &Vec<Box>, luminosity: u16) {
    leds.iter_mut().for_each(|l| l.update_color(boxes, luminosity));
}


fn write_to_serial(leds: &Vec<Led>, port: &mut serialport::TTYPort, led_groups_pop: u8, start_corner: u8, x_led_count: u16, y_led_count: u16) {
    let mut values: Vec<u8> = Vec::new();
    values.push(255); //255 means start of leds sequence
    values.push(253); //253 means that next byte is led_group_pop.
    values.push(led_groups_pop);

    let starting_led = match start_corner {
        0 => 0,
        1 => x_led_count,
        2 => x_led_count + y_led_count,
        3 => 2 * x_led_count + y_led_count,
        _ => 0
    };

    for idx in 0..leds.len() {
            values.push(254); //254 means start of led 
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + usize::from(starting_led)) % leds.len()].r).unwrap(), 252)); //led color is
                                                                                    //capped at 252
                                                                                    //to allow for
                                                                                    //253, 254, and
                                                                                    //255 values to
                                                                                    //    be
                                                                                    //    headers.
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + usize::from(starting_led)) % leds.len()].g).unwrap(), 252)); //led color is
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + usize::from(starting_led)) % leds.len()].b).unwrap(), 252)); //led color is
        }


    match port.write(&values[..]) {
        Ok(_) => {
            //print!("{}", &string);
            std::io::stdout().flush().unwrap();
        }
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn main() {
    //hardware parameters
    let x_res = 2560;
    let y_res = 1440;
    let x_led_count = 82;
    let y_led_count = 47;
    let start_corner = 0;
    let serial_port = "/dev/ttyUSB0";

    //preferences parameters
    let luminosity_percent = 20;
    let mean_width= 300;
    let mean_depth = 300;
    let random_sampling = false;


    //performance parameters
    let x_box_cnt = 8;
    let y_box_cnt = 4;
    let sampling_size = 10;
    let led_groups_pop = 1;
    let loop_min_time = 30;
    let baud_rate = 576_000;



    let mut boxes = get_boxes(x_res, y_res, x_box_cnt, y_box_cnt, mean_depth, mean_width / 2, sampling_size, random_sampling);
    let mut leds = get_leds(&boxes, x_led_count, y_led_count, x_res, y_res, led_groups_pop);

    let screen = Screen::open().unwrap();

    let mut port = serialport::new(serial_port, baud_rate)
        .open_native().expect("Failed to open port");


    loop {
        let start = Instant::now();
        color_boxes(&mut boxes, &screen, sampling_size, random_sampling);
        color_leds(&mut leds, &boxes, luminosity_percent);
        write_to_serial(&leds, &mut port, led_groups_pop, start_corner, x_led_count, y_led_count);
        let duration = start.elapsed();
        thread::sleep(Duration::from_millis(loop_min_time).saturating_sub(duration));
    }
}

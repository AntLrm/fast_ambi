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
    led_groups_pop: u16
) -> Vec<Led> {

    let x_group_count: u16 = x_led_count / led_groups_pop;
    let y_group_count: u16 = y_led_count / led_groups_pop;

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
    fn update_color(&mut self, boxes: &Vec<Box>, luminosity: u16, luminosity_fading: bool, luminosity_fading_speed: u16) {
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

        let mut luminosity_fading_factor = 100;
        let target_r = luminosity * ((relative_unit * b2.r + (100 - relative_unit) * b1.r) / 100) / 100;
        let target_g = luminosity * ((relative_unit * b2.g + (100 - relative_unit) * b1.g) / 100) / 100;
        let target_b = luminosity * ((relative_unit * b2.b + (100 - relative_unit) * b1.b) / 100) / 100;

        if luminosity_fading {
            let current_luminosity: u16 = (self.r as u16 + self.g as u16 + self.b as u16) / 3;
            let target_luminosity: u16 = (target_r + target_g + target_b) / 3;
            let diff_lum: i32 = target_luminosity as i32 - current_luminosity as i32;
            luminosity_fading_factor = match target_luminosity {
                0 => 0,
                _ => cmp::max(
                0,

                (       
                    (
                        (current_luminosity * 100) as i32 +
                        100 * (diff_lum).signum() * (cmp::min(luminosity_fading_speed, target_luminosity.abs_diff(current_luminosity))) as i32
                    ) as u32
                ) / target_luminosity as u32

            )};
            if diff_lum > 40 {
            println!("---");
            println!("{} {} {}", self.r, self.g, self.b);
            println!("{}", current_luminosity);
            println!("{} {} {}", target_r, target_g, target_b);
            println!("{}", target_luminosity);
            println!("{} {}", (diff_lum).signum(), (target_luminosity.abs_diff(current_luminosity)));
            println!("{}", luminosity_fading_factor);
            }
        };

        self.r = (luminosity_fading_factor * target_r as u32 /100) as u8;
        self.g = (luminosity_fading_factor * target_g as u32 /100) as u8;
        self.b = (luminosity_fading_factor * target_b as u32 /100) as u8;
    }
}

fn color_leds(leds: &mut Vec<Led>, boxes: &Vec<Box>, luminosity: u16, luminosity_fading: bool, luminosity_fading_speed: u16) {
    leds.iter_mut().for_each(|l| l.update_color(boxes, luminosity, luminosity_fading, luminosity_fading_speed));
}


fn write_to_serial(leds: &Vec<Led>, port: &mut serialport::TTYPort) {
    let mut values: Vec<u8> = Vec::new();
    //values.push(b'u');
    values.push(255);

    leds.iter()
        .for_each(|l| {
            values.push(254);
            values.push(std::cmp::min(TryInto::<u8>::try_into(l.r).unwrap(), 253));
            values.push(std::cmp::min(TryInto::<u8>::try_into(l.g).unwrap(), 253));
            values.push(std::cmp::min(TryInto::<u8>::try_into(l.b).unwrap(), 253));
        }
        );


    //println!("{:?}", &values[..]);
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
    let x_res = 2560;
    let y_res = 1440;
    let luminosity_percent = 20;
    let x_box_cnt = 8;
    let y_box_cnt = 4;
    let mean_radius = 150;
    let mean_depth = 300;
    let x_led_count = 82;
    let y_led_count = 47;
    let sampling_size = 10; //number of pixel for mean color calculation per boxes. Increase it
                            //will improve color accuracy at the cost of reactivity.
    let random_sampling = false; //If set to true, will randomly select different pixels for each loop. Setting it to true will make the color vary even when screen is not changing, especially if sampling size is low.
    let led_groups_pop = 1; //bugged feature for grouping leds. Keep it to 1.
    let luminosity_fading = false; //bugged feature for fading luminosity to avoid abrupt changes.
                                   //TODO fix it. Keep it to false otherwise.
    let luminosity_fading_speed = 30;
    let loop_min_time = 30; //loop minimum time in millis(). Decreasing will increase reactivity
                            //but increase cpu usage.
    

    let mut boxes = get_boxes(x_res, y_res, x_box_cnt, y_box_cnt, mean_depth, mean_radius, sampling_size, random_sampling);
    let mut leds = get_leds(&boxes, x_led_count, y_led_count, x_res, y_res, led_groups_pop);

    let screen = Screen::open().unwrap();

    let mut port = serialport::new("/dev/ttyUSB0", 576_000)
        .open_native().expect("Failed to open port");

    println!{"Port open"};

    loop {
        let start = Instant::now();
        color_boxes(&mut boxes, &screen, sampling_size, random_sampling);
        color_leds(&mut leds, &boxes, luminosity_percent, luminosity_fading, luminosity_fading_speed);
        write_to_serial(&leds, &mut port);
        let duration = start.elapsed();
        thread::sleep(Duration::from_millis(loop_min_time).saturating_sub(duration));
    }
}

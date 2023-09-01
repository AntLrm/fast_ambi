use rand::Rng;
use std::time::{Duration, Instant};
use std::cmp;
use std::ptr;
use std::thread;
use x11::xlib;
use std::io::{self, Write};

#[derive(Debug)]
enum ScreenSide {
    Top,
    Right,
    Bottom,
    Left
}

impl ScreenSide {
    fn get_offset(&self, screen: &Screen) -> u32 {
        match self {
            ScreenSide::Top     => 0,
            ScreenSide::Right   => screen.x_res,
            ScreenSide::Bottom  => screen.x_res + screen.y_res,
            ScreenSide::Left    => 2 * screen.x_res + screen.y_res
        }
    }

    fn get_side_res(&self, screen: &Screen) -> u32 {
        match self {
            ScreenSide::Top | ScreenSide::Bottom    => screen.x_res,
            ScreenSide::Right | ScreenSide::Left    => screen.y_res,
        }
    }

    fn get_linear_from_border_coord(&self, screen: &Screen, border_coord: u32) -> u32 {
        match self {
            ScreenSide::Top | ScreenSide::Right     =>  self.get_offset(screen) + border_coord,
            ScreenSide::Bottom | ScreenSide::Left   =>  self.get_offset(screen) + self.get_side_res(screen) - border_coord - 1,
        }
    }
}


#[derive(Debug)]
struct CRGB {
    r: u8,
    g: u8,
    b: u8
}


#[derive(Debug)]
struct ScreenCoord {
    x: u32,
    y: u32,
}


impl ScreenCoord {
    fn convert_to_linear_coord(&self, side: &ScreenSide, screen: &Screen) -> u32 {
        match side {
            ScreenSide::Top | ScreenSide::Bottom    => side.get_linear_from_border_coord(screen, self.x),
            ScreenSide::Right | ScreenSide::Left    => side.get_linear_from_border_coord(screen, self.y),
        }
    }
}

fn get_random_sampling(b: &Box) -> ScreenCoord {
    ScreenCoord {
        x: rand::thread_rng().gen_range(b.screen_start.x..b.screen_end.x),
        y: rand::thread_rng().gen_range(b.screen_start.y..b.screen_end.y),
    }
}

fn get_side_from_linear(linear: u32, screen: &Screen) -> ScreenSide {
    if      linear < ScreenSide::Right.get_offset(screen) {
        return ScreenSide::Top;
    }
    else if linear < ScreenSide::Bottom.get_offset(screen) {
        return ScreenSide::Right;
    }
    else if linear < ScreenSide::Left.get_offset(screen) {
        return ScreenSide::Bottom;
    }
    else {
        return ScreenSide::Left;
    }

}

fn convert_linear_coord_to_screen_coord(linear: u32, screen: &Screen, depth: u32) -> ScreenCoord {
    let current_side = get_side_from_linear(linear, screen);

    let x = match current_side {
            ScreenSide::Top     => linear,
            ScreenSide::Right   => screen.x_res - 1 - depth,
            ScreenSide::Bottom  => screen.x_res - (linear - current_side.get_offset(screen)) - 1,
            ScreenSide::Left    => depth
        };

        let y = match current_side {
            ScreenSide::Top     => depth,
            ScreenSide::Right   => linear - current_side.get_offset(screen),
            ScreenSide::Bottom  => screen.y_res - 1 - depth,
            ScreenSide::Left    => screen.y_res - (linear - current_side.get_offset(screen)) - 1,
        };

        ScreenCoord {
            x,
            y
        }
}

#[derive(Debug)]
struct Box {
    screen_start: ScreenCoord,
    screen_end: ScreenCoord,
    sample_points : Vec<ScreenCoord>,
    color: CRGB,
    side: ScreenSide,
}

struct Led {
    box_idx: usize,
    linear_position: u32,
    color: CRGB,
}

impl Box {
    fn set_color_from_rgb_vec(&mut self, color_vec: Vec<u32>) {
        self.color = CRGB {
            r: color_vec[0] as u8,
            g: color_vec[1] as u8,
            b: color_vec[2] as u8,
        }
    }

    fn get_linear_coord(&self, screen: &Screen) -> (u32, u32) {
        let box_linear_start = self.screen_start.convert_to_linear_coord(&self.side, screen);
        let box_linear_end = self.screen_end.convert_to_linear_coord(&self.side, screen);
        (cmp::min(box_linear_start, box_linear_end),  cmp::max(box_linear_start, box_linear_end))
    }
}

pub struct Screen {
    display: *mut xlib::Display,
    window: xlib::Window,
    x_res: u32,
    y_res: u32
}

impl Screen {
    pub fn open() -> Option<Screen> {
        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());
            if display.is_null() {
                return None;
            }
            let screen= xlib::XDefaultScreenOfDisplay(display);
            let root = xlib::XRootWindowOfScreen(screen);
            Some(Screen {
                display,
                window: root,
                x_res:  (*screen).width as u32,
                y_res:  (*screen).height as u32,
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


fn switch_corners(corner_1: ScreenCoord, corner_2: ScreenCoord) -> (ScreenCoord, ScreenCoord) {
    (ScreenCoord {
        x: cmp::min(corner_1.x, corner_2.x),
        y: cmp::min(corner_1.y, corner_2.y),
    },
    ScreenCoord {
        x: cmp::max(corner_1.x, corner_2.x),
        y: cmp::max(corner_1.y, corner_2.y),
    })
}

fn new_box(
    idx: u32,
    box_cnt: u32,
    linear_width: u32,
    side: ScreenSide,
    box_screen_depth: u32,
    screen: &Screen
) -> Box {
    
    let linear_start = side.get_offset(screen) + (idx * linear_width);

    let linear_end      = match idx.cmp(&(box_cnt - 1)) {
        cmp::Ordering::Equal    => Ok(side.get_side_res(screen) + side.get_offset(screen)  -  1),
        cmp::Ordering::Less     => Ok(side.get_offset(screen)   + (idx + 1) * linear_width -  1),
        cmp::Ordering::Greater  => Err("idx more than box cnt"),
        }.unwrap();

    let screen_start = convert_linear_coord_to_screen_coord(linear_start, screen, 0);
    let screen_end = convert_linear_coord_to_screen_coord(linear_end, screen, box_screen_depth);
    let (screen_start, screen_end) = switch_corners(screen_start, screen_end);

    Box {
        screen_start,
        screen_end,
        sample_points: Vec::new(),
        color: CRGB { r: 0, g: 0, b: 0, },
        side,
    }
}


fn new_led(boxes: &Vec<Box>, linear_position: u32, screen: &Screen) -> Led {

    let (idx, _) = boxes
        .iter()
        .enumerate()
        .find(|(_, b)| b.get_linear_coord(screen).0 <= linear_position && b.get_linear_coord(screen).1 >= linear_position)
        .unwrap();

    Led {
        box_idx: idx,
        linear_position,
        color: CRGB {
        r: 0,
        g: 0,
        b: 0,
        }
    }
}

fn set_regular_sampling_points(boxes: &mut Vec<Box>, sampling_size: u32) {
    boxes.iter_mut()
        .for_each(|b| {
            (0..sampling_size)
                .collect::<Vec<u32>>()
                .iter()
                .for_each(|_| {
                    b.sample_points.push(get_random_sampling(b));
                })
        })
}



fn get_boxes(
    screen: &Screen,
    x_box_cnt: u32,
    y_box_cnt: u32,
    boxes_linear_depth: u32,
    sampling_size: u32,
    random_sampling: bool
) -> Vec<Box> {
    let box_cnt: usize = 2 * (x_box_cnt as usize + y_box_cnt as usize);
    let mut boxes = Vec::with_capacity(box_cnt);
    let x_width: u32;
    let y_width: u32;
    
    x_width = screen.x_res / x_box_cnt;
    y_width = screen.y_res / y_box_cnt;

    let x_width = x_width;
    let y_width = y_width;

    //top boxes
    for idx in 0..x_box_cnt {
        boxes.push(new_box(
            idx,
            x_box_cnt,
            x_width,
            ScreenSide::Top,
            boxes_linear_depth,
            screen
        ));
    }

    //right boxes
    for idx in 0..y_box_cnt {
        boxes.push(new_box(
            idx,
            y_box_cnt,
            y_width,
            ScreenSide::Right,
            boxes_linear_depth,
            screen
        ));
    }

    //bottom boxes
    for idx in 0..x_box_cnt {
        boxes.push(new_box(
            idx,
            x_box_cnt,
            x_width,
            ScreenSide::Bottom,
            boxes_linear_depth,
            screen
        ));
    }

    //left boxes
    for idx in 0..y_box_cnt {
        boxes.push(new_box(
            idx,
            y_box_cnt,
            y_width,
            ScreenSide::Left,
            boxes_linear_depth,
            screen
        ));
    }

    if !random_sampling {
        set_regular_sampling_points(&mut boxes, sampling_size);
    }

    return boxes;
}

fn get_leds(
    boxes: &Vec<Box>,
    x_led_count: u32,
    y_led_count: u32,
    screen: &Screen
) -> Vec<Led> {


    let mut leds = Vec::with_capacity(2 * (x_led_count + y_led_count) as usize);

    //top leds
    for idx in 0..x_led_count {
        let led_pos_on_border: u32 = (screen.x_res  / x_led_count) * idx;
        let led_linear_pos = ScreenSide::Top.get_linear_from_border_coord(screen, led_pos_on_border);
        leds.push(new_led(&boxes, led_linear_pos, screen));
    }

    //right leds
    for idx in 0..y_led_count {
        let led_pos_on_border: u32 = (screen.y_res / y_led_count) * idx;
        let led_linear_pos = ScreenSide::Right.get_linear_from_border_coord(screen, led_pos_on_border);
        leds.push(new_led(&boxes, led_linear_pos, screen));
    }

    //bottom leds
    for idx in (0..x_led_count).rev() {
        let led_pos_on_border: u32 = (screen.x_res / x_led_count) * idx;
        let led_linear_pos = ScreenSide::Bottom.get_linear_from_border_coord(screen, led_pos_on_border);
        leds.push(new_led(&boxes, led_linear_pos, screen));
    }

    //left leds
    for idx in (0..y_led_count).rev() {
        let led_pos_on_border: u32 = (screen.y_res / y_led_count) * idx;
        let led_linear_pos = ScreenSide::Left.get_linear_from_border_coord(screen, led_pos_on_border);
        leds.push(new_led(&boxes, led_linear_pos, screen));
    }

    return leds;
}

fn color_boxes(boxes: &mut Vec<Box>, screen: &Screen, sample_count: u32, random_sampling: bool) {
    boxes.iter_mut().for_each(|b| {
        let mean_color: Vec<u32> = (0..sample_count)
            .collect::<Vec<u32>>()
            .iter()
            .map(|&idx| {
                let sampling_pixel;
                if random_sampling {
                    sampling_pixel = get_random_sampling(b);
                } else {
                    sampling_pixel = ScreenCoord {
                        x: b.sample_points[idx as usize].x,
                        y: b.sample_points[idx as usize].y,
                    }
                }

                let screen_capture = screen.capture_ximage(1, 1, sampling_pixel.x as i32, sampling_pixel.y as i32);
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
            .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32])
            .fold([0, 0, 0], |[rs, gs, bs], [r, g, b]| {
                [rs + r, gs + g, bs + b]
            })
            .iter()
            .map(|c| c / sample_count)
            .collect();

        b.set_color_from_rgb_vec(mean_color);
    });
}

impl Led {
    fn update_color(&mut self, boxes: &Vec<Box>, screen: &Screen, luminosity: u32, color_correction_red: u32, color_correction_green: u32, color_correction_blue: u32) {
        let boxes_last_idx= boxes.len() - 1;
        let current_box = &boxes[usize::from(self.box_idx)];
        let next_box = &boxes[(self.box_idx + 1) % boxes.len()];
        let previous_box = &boxes[self.box_idx.checked_sub(1).unwrap_or(boxes_last_idx)];
        let current_box_linear = current_box.get_linear_coord(screen);
        let current_box_center = current_box_linear.0 + (current_box_linear.1 - current_box_linear.0) / 2;

        let (b1, b2, relative_unit) = match self.linear_position.cmp(&current_box_center) {
            cmp::Ordering::Less                             => (previous_box, current_box, 100 * (self.linear_position - current_box_linear.0)  / (current_box_center - current_box_linear.0)),
            cmp::Ordering::Greater | cmp::Ordering::Equal   => (current_box, next_box, 100 * (self.linear_position - current_box_center)  / (current_box_linear.1 - current_box_center)),
        };

        self.color = CRGB {
            r: (luminosity * (color_correction_red   * ((relative_unit * b2.color.r as u32 + (100 - relative_unit) * b1.color.r as u32) / 100) /100) /100) as u8,
            g: (luminosity * (color_correction_green   * ((relative_unit * b2.color.g as u32 + (100 - relative_unit) * b1.color.g as u32) / 100) /100) /100) as u8,
            b: (luminosity * (color_correction_blue   * ((relative_unit * b2.color.b as u32 + (100 - relative_unit) * b1.color.b as u32) / 100) /100) /100) as u8,
        };
    }
}

fn color_leds(leds: &mut Vec<Led>, boxes: &Vec<Box>, screen: &Screen, luminosity: u32, color_correction_red: u32, color_correction_green: u32, color_correction_blue: u32) {
    leds.iter_mut().for_each(|l| l.update_color(boxes, screen, luminosity, color_correction_red, color_correction_green, color_correction_blue));
}


fn write_to_serial(leds: &Vec<Led>, port: &mut serialport::TTYPort, start_corner: u8, x_led_count: u32, y_led_count: u32) {
    let mut values: Vec<u8> = Vec::new();
    values.push(255); //255 means start of leds sequence

    let starting_led = match start_corner {
        0 => 0,
        1 => x_led_count,
        2 => x_led_count + y_led_count,
        3 => 2 * x_led_count + y_led_count,
        _ => 0
    };


    
    for idx in 0..leds.len() {
            values.push(254); //254 means start of led 
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + starting_led as usize) % leds.len()].color.r).unwrap(), 252)); //led color is
                                                                                    //capped at 252
                                                                                    //to allow for
                                                                                    //253, 254, and
                                                                                    //255 values to
                                                                                    //    be
                                                                                    //    headers.
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + starting_led as usize) % leds.len()].color.g).unwrap(), 252)); //led color is
            values.push(std::cmp::min(TryInto::<u8>::try_into(leds[(idx + starting_led as usize) % leds.len()].color.b).unwrap(), 252)); //led color is
        }


    match port.write(&values[..]) {
        Ok(_) => {
            std::io::stdout().flush().unwrap();
        }
        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
        Err(e) => eprintln!("{:?}", e),
    }
}

fn main() {
    //hardware parameters
    let x_led_count = 82;
    let y_led_count = 47;
    let start_corner = 0;
    let serial_port = "/dev/ttyUSB0";

    //preferences parameters
    let luminosity_percent = 20;
    let border_depth = 200; 
    let random_sampling = false;
    let color_correction_red = 120;
    let color_correction_green = 100;
    let color_correction_blue = 100;


    //performance parameters
    let x_box_cnt = 9;
    let y_box_cnt = 5;
    let sampling_size = 10;
    let loop_min_time = 30;
    let baud_rate = 576_000;

    let screen = Screen::open().unwrap();

    let mut boxes = get_boxes(&screen, x_box_cnt, y_box_cnt, border_depth, sampling_size, random_sampling);
    let mut leds = get_leds(&boxes, x_led_count, y_led_count, &screen);
    let mut port = serialport::new(serial_port, baud_rate)
        .open_native().expect("Failed to open port");


    loop {
        let start = Instant::now();
        color_boxes(&mut boxes, &screen, sampling_size, random_sampling);
        color_leds(&mut leds, &boxes, &screen, luminosity_percent, color_correction_red, color_correction_green, color_correction_blue);
        write_to_serial(&leds, &mut port, start_corner, x_led_count, y_led_count);
        let duration = start.elapsed();
        thread::sleep(Duration::from_millis(loop_min_time).saturating_sub(duration));
    }
}

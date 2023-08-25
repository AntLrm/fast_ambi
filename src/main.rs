use std::cmp;
use rand::Rng;
use screenshots::Screen;
use image;

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
    b: u16
}

impl Box {
    fn set_color(&mut self, r: u16, g: u16, b: u16){
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
    b: u16
}

fn new_box(idx: u16, width: u16, side: u16, mean_radius: u16, mean_depth: u16, res_in_dir: u16, res_ortho: u16) -> Box {
    let section_center: u16 = (idx * width) + width / 2;
    let in_dir_mean_min = (u16::MIN + section_center).checked_sub(mean_radius).unwrap_or(0);
    let in_dir_mean_max = cmp::min(section_center + mean_radius, res_in_dir);

    Box {
        section_start: idx * width,
        section_end: (idx + 1) * width - 1,
        side,
        mean_x_min: match side {
            0 | 2 => in_dir_mean_min,
            1 => res_ortho - mean_depth - 1,
            3 => 0,
            _ => 0
        },
        mean_x_max: match side {
            0 | 2 => in_dir_mean_max,
            1 => res_ortho - 1,
            3 => mean_depth - 1,
            _ => 0
        },
        mean_y_min: match side {
            0 => 0,
            2 => res_ortho - mean_depth - 1,
            1 | 3 => in_dir_mean_min,
            _ => 0
        },
        mean_y_max: match side {
            0 => mean_depth - 1,
            2 => res_ortho - 1,
            1 | 3 => in_dir_mean_max,
            _ => 0
        },
        r: 0,
        g: 0,
        b: 0
    }
}

fn new_led(boxes: &Vec<Box>, led_pos: u16, side: u16) -> Led {
    let (idx, bx) = boxes.iter()
                 .enumerate()
                 .find(|(i, b)| {b.section_start <= led_pos && b.section_end >= led_pos && b.side == side})
                 .unwrap();
        
    Led {
        box_idx: idx as u16,
        relative_box_position: (led_pos - bx.section_start) * 100 / (bx.section_end - bx.section_start),
        r: 0,
        g: 0,
        b: 0
    }
}

fn get_boxes(res_x: u16, res_y: u16, x_box_cnt: u16, y_box_cnt: u16, mean_depth: u16, mean_radius: u16) -> Vec<Box> {
    let box_cnt: u16 = 2 * (x_box_cnt + y_box_cnt);
    let mut boxes = Vec::with_capacity(usize::from(box_cnt));
    let x_width: u16;
    let y_width: u16;
    x_width = res_x / x_box_cnt;
    y_width = res_y / y_box_cnt;

    //top boxes
    for idx in 0..x_box_cnt {
        boxes.push(new_box(idx, x_width, 0, mean_radius, mean_depth, res_x, res_y));
    }

    //right boxes
    for idx in 0..y_box_cnt {
        boxes.push(new_box(idx, y_width, 1, mean_radius, mean_depth, res_y, res_x));
    }

    //bottom boxes
    for idx in (0..x_box_cnt).rev() {
        boxes.push(new_box(idx, x_width, 2, mean_radius, mean_depth, res_x, res_y));
    }

    //left boxes
    for idx in (0..y_box_cnt).rev() {
        boxes.push(new_box(idx, y_width, 3, mean_radius, mean_depth, res_y, res_x));
    }

    return boxes
}

fn get_leds(boxes: &Vec<Box>, x_led_count: u16, y_led_count: u16, res_x: u16, res_y: u16) -> Vec<Led> {
   let mut leds = Vec::with_capacity(usize::from(2* (x_led_count + y_led_count)));

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

   return leds
}

fn color_boxes(boxes: &mut Vec<Box>, screen_capture: image::RgbaImage, sample_count: u16) {
    boxes.iter_mut()
        .for_each(|b| {
           let mean_color: Vec<u16> = (0..sample_count).collect::<Vec<u16>>()
                                        .iter()
                                        .map(|_| {
                                            screen_capture.get_pixel(rand::thread_rng().gen_range(b.mean_x_min..b.mean_x_max).into(), rand::thread_rng().gen_range(b.mean_y_min..b.mean_y_max).into())
                                        })
                                        .map(|c| [c[0] as u16, c[1] as u16, c[2] as u16])
                                        .fold([0, 0, 0], |[rs, gs, bs], [r,g,b]| [rs+r, gs + g, bs + b])
                                        .iter()
                                        .map(|c| c / sample_count)
                                        .collect();
            b.set_color(mean_color[0], mean_color[1], mean_color[2]);
        });
}

impl Led{
    fn update_color(&mut self, boxes: &Vec<Box>) {
        self.r = boxes[usize::from(self.box_idx)].r;
        self.g = boxes[usize::from(self.box_idx)].g;
        self.b = boxes[usize::from(self.box_idx)].b;
    }
}


fn main() {
    //read_config();
    let mut boxes = get_boxes(2560, 1440, 20, 20, 80, 80);
    let mut leds = get_leds(&boxes, 86, 35, 2560, 1440);

    while true {
        println!{"START"}
        let screens = Screen::all().unwrap();
        for screen in screens {
            let image = screen.capture().unwrap();
            color_boxes(&mut boxes, image, 30);
            leds.iter_mut()
                .for_each(|l| l.update_color(&boxes));
            leds.iter()
                .for_each(|b| {println!("{} {} {}", b.r, b.g, b.b)});
        }
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

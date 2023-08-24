use std::cmp;

struct Box {
    section_start: u16,
    section_end: u16,
    side: u8, //0 top, 1 right, 2 bottom, 3 left
    mean_x_min: u16,
    mean_x_max: u16,
    mean_y_min: u16,
    mean_y_max: u16,
    r: u8,
    g: u8,
    b: u8
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
        let section_center: u16 = (idx * x_width) + x_width / 2;
        boxes.push(Box {
            section_start: idx * x_width,
            section_end: (idx + 1) * x_width - 1,
            side: 0,
            mean_x_min: (u16::MIN + section_center).checked_sub(mean_radius).unwrap_or(0),
            mean_x_max: cmp::min(section_center + mean_radius, res_x),
            mean_y_min: 0,
            mean_y_max: mean_depth,
            r: 0,
            g: 0,
            b: 0
        });
    };

    //right boxes
    for idx in 0..y_box_cnt {
        let section_center: u16 = (idx * y_width) + y_width / 2;
        boxes.push(Box {
            section_start: idx * y_width,
            section_end: (idx + 1) * y_width - 1,
            side: 1,
            mean_x_min: res_x - mean_depth,
            mean_x_max: res_x,
            mean_y_min: (u16::MIN + section_center).checked_sub(mean_radius).unwrap_or(0),
            mean_y_max: cmp::min(section_center +mean_radius, res_y),
            r: 0,
            g: 0,
            b: 0
        });
    };

    //bottom boxes
    for idx in (0..x_box_cnt).rev() {
        let section_center: u16 = (idx * x_width) + x_width / 2;
        boxes.push(Box {
            section_start: idx * x_width,
            section_end: (idx + 1) * x_width - 1,
            side: 2,
            mean_x_min: (u16::MIN + section_center).checked_sub(mean_radius).unwrap_or(0),
            mean_x_max: cmp::min(section_center + mean_radius, res_x),
            mean_y_min: 0,
            mean_y_max: mean_depth,
            r: 0,
            g: 0,
            b: 0
        });
    };

    //left boxes
    for idx in (0..y_box_cnt).rev() {
        let section_center: u16 = (idx * y_width) + y_width / 2;
        boxes.push(Box {
            section_start: idx * y_width,
            section_end: (idx + 1) * y_width - 1,
            side: 1,
            mean_x_min: res_x - mean_depth,
            mean_x_max: res_x,
            mean_y_min: (u16::MIN + section_center).checked_sub(mean_radius).unwrap_or(0),
            mean_y_max: cmp::min(section_center +mean_radius, res_y),
            r: 0,
            g: 0,
            b: 0
        });
    };

    return boxes
}



fn main() {
    //read_config();
    let boxes = get_boxes(2660, 1440, 7, 5, 100, 500);
    let leds = get_leds(boxes, 86, 35);

    boxes.iter()
        .for_each(|b| {println!("{} {} {} {} {} {} {}", b.section_start, b.section_end, b.mean_x_min, b.mean_y_min, b.mean_x_max, b.mean_y_max, b.side)});
}

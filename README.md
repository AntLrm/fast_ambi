# FAST-AMBI
An ambilight implementation for Arduino controlled led strips and X11 desktops written in Rust.
Ambilight setups are led strips around the back of a computer or a TV screen that reproduce colors displayed on the screen.

img


This implementation is inspired by Ambilight X11 project but with some tweaks to reduce delay between image displayed, and leds color update.

The program works by computing leds color using X11 screen capture on specific pixels and sending color info through serial to an arduino that control the leds strips with FastLED library.

# How it works

# Installation

# Usage

# Configuration
You should experiment with different setting to find the numbers that work best for your hardware and your preferences. You can improve color accuracy and definition at the cost of CPU usage and delay.

Delay is due to 3 bottlenecks (depending on your hardware) :
* Screen capture, which can be pretty slow with X11. This can be mitigated by reducing the number of screen sample at the cost of color accuracy.
* Artificial loop frequency limiter that can be reduce or even disable at the cost of CPU usage. 
* Serial communication. This can be mitigated by increasing serial baud rate if your Arduino can handle it, and reducing data send by grouping leds together.

Here are the different parameters and what they do:
## Hardware settings:
* *x_res*: horizontal resolution of your screen
* *y_res*: vertical resolution of your screen
* *x_led_count*: Number of leds around the horizontal axis.
* *y_led_count*: Number of leds around the vertical axis.

## Performance settings:
* *x_box_cnt*: Number of squares used for mean color calculation on the horizontal axis. Increazing this number will increase precision and resolution of colors on the horizontal axis but will increase delay as more screen capture will be necessary.
* *y_box_cnt*: Same thing as the x_box_cnt on the vertical axis.
* *sampling_size*: Number of pixels sampled for each box. Increasing it will improve color accuracy at the cost of delay.
* *led_groups_pop*: Allow for grouping of neighbour leds together that will share the same color in order to reduce the size of the data sent through serial and reduce delay. Increasing it will reduce delay but decrease color definition.
* *loop_min_time*: Loop time limiter, in milliseconds. Increasing this parameter will reduce CPU usage at the cost of delay. Set to 0 will minimize delay and maximize CPU usage.


## Preference settings:
* *luminosity_percent*: luminosity of the leds in percentage.
* *mean_radius*: Box size on the axis of the side of the screen in pixel. (Boxes can overlap). Usually around the number of pixel of your screen along an axis divided by the number of boxes around this axis. Increasing it will blend the colors together more.
* *mean_depth*: Box size on the axis perpendicular to the side of the screen. Increasing will make the software pick colors more towards the center of the screen, while decreasing it will make the color sampling stay on the borders.
* *random_sampling*: When on, this will change the pixels sampled at each loop randomly, making the colors change slightly even when the image is still, especially if sampling_size is low. Usually kept off.

* *luminosity_fading*:
* *luminosity_fading_speed*:



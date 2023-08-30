#include <FastLED.h>
#define NUM_LEDS 258
#define DATA_PIN 2
#define TIMEOUT 1000

CRGB leds[NUM_LEDS];
CRGB back;

bool at_start = false;
bool flash;
long mtime;
long itime;
byte led_group_pop;
bool timeout = false;

byte readserial() {
  unsigned long now = millis();
  while (Serial.available() <= 0) {
    if ((millis() - now) > TIMEOUT) {
      turnoffleds();
      timeout = true;
      return 0;
    }
  }
  return Serial.read();
}

void turnoffleds() {
  back = CRGB(0,0,0);

  for (int i = 0; i < NUM_LEDS; i ++){
    leds[i] = back;
  }
  
  FastLED.show();
}

void setup() { 
  Serial.begin(576000);
  
  FastLED.addLeds<NEOPIXEL, DATA_PIN>(leds, NUM_LEDS);
  //FastLED.setMaxRefreshRate(0);

  turnoffleds();
  
}

void loop() {
  if (at_start) {
    at_start = false;
    
    for (int i = 0; i < NUM_LEDS; i ++){
      byte l = 0;
      byte rt, gt, bt;
      
      
      while (l != 254) {
        l = readserial();
        if (l == 255) {
          at_start = true;
          return;
        } else if (timeout) {
          at_start = false;
          return;
        }
      }

      
      rt = readserial();
      if (rt == 255) {
        at_start = true;
        return;
      } else if (timeout) {
        at_start = false;
        return;
      }
      
    
      bt = readserial();
      if (bt == 255) {
        at_start = true;
        return;
      } else if (timeout) {
        at_start = false;
        return;
      }
    
      gt = readserial();
      if (gt == 255) {
        at_start = true;
        return;
      } else if (timeout) {
        at_start = false;
        return;
      }

      
        leds[i].r = rt;
        leds[i].g = gt;
        leds[i].b = bt;
      
    }   
    FastLED.show();
  }


  byte c = 0;
  while (c != 255) {
    c = readserial();
  }
  at_start = true;
}

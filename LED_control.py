import os
import glob
import time
import sys
sys.path.append('./env/lib/python3.11/site-packages/rpi5_ws2812/')

from rpi5_ws2812.ws2812 import Color, WS2812SpiDriver

# Initialize temperature sensor
os.system('modprobe w1-gpio')
os.system('modprobe w1-therm')

base_dir = '/sys/bus/w1/devices/'
device_folder = glob.glob(base_dir + '28*')[0]
device_file = device_folder + '/w1_slave'

# Initialize LED strip with 10 LEDs
strip = WS2812SpiDriver(spi_bus=0, spi_device=0, led_count=10).get_strip()

# Temperature range for LED control
MIN_TEMP = 40
MAX_TEMP = 50

def read_temp_raw():
    with open(device_file, 'r') as f:
        lines = f.readlines()
    return lines

def read_temp():
    lines = read_temp_raw()
    while lines[0].strip()[-3:] != 'YES':
        time.sleep(0.2)
        lines = read_temp_raw()
    equals_pos = lines[1].find('t=')
    if equals_pos != -1:
        temp_string = lines[1][equals_pos+2:]
        temp_c = float(temp_string) / 1000.0
        return temp_c
    return None

def update_leds(temp):
    step = (MAX_TEMP - MIN_TEMP) / 10  # 计算每颗灯珠的温度区间
    brightness_step = 255 / step  # 计算每升高1度亮度增加量
    
    led_count = min(10, max(0, int((temp - MIN_TEMP) / step) + 1))
    remainder = (temp - MIN_TEMP) % step
    last_led_brightness = min(255, int(remainder * brightness_step))
    
    def get_color(brightness):
        if brightness == 255:
            return Color(255, 0, 0)  # 已满亮度的灯珠为红色
        else:
            r = int((brightness / 255) * 255)  # 逐渐从蓝色变为红色
            b = int((1 - brightness / 255) * 255)
            return Color(r, 0, b)
    
    if temp > MAX_TEMP:
        flash_colors = [Color(255, 255, 255), Color(127, 127, 127)]
        for i in range(5):  # 2Hz 闪烁效果
            strip.set_pixel_color(9, flash_colors[i % 2])
            strip.show()
            time.sleep(0.5)
        return
    
    if temp < MIN_TEMP:
        flash_colors = [Color(127, 127, 127), Color(0, 0, 0)]
        for i in range(5):  # 2Hz 闪烁效果
            strip.set_pixel_color(0, flash_colors[i % 2])
            strip.show()
            time.sleep(0.5)
        return
    
    for i in range(10):
        if i < led_count - 1:
            strip.set_pixel_color(i, Color(255, 0, 0))  # 完全点亮的灯珠为红色
        elif i == led_count - 1:
            strip.set_pixel_color(i, get_color(last_led_brightness))  # 新点亮的灯珠从蓝到红渐变
        else:
            strip.set_pixel_color(i, Color(0, 0, 0))  # 熄灭的灯珠
    
    strip.show()

while True:
    temperature = read_temp()
    if temperature is not None:
        print(f"Temperature: {temperature:.2f}°C")
        update_leds(temperature)
    time.sleep(1)

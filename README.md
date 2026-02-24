# Rocket Viewer
This is the telemetry viewer for the Cascadia Engineering club model rocket project
it takes input from the serial monitor in the form of 1 line json strings and displays the data on the screen

# To Run:
-Make sure you have rust installed (can be installed here[here](https://rust-lang.org/))
-navigate to project folder in terminal and run it normally

'''shell
$ cargo run
'''

# Notes
currently project only works on com3 port at 9600 baud, this is because I set it up to receive serial data from an esp32 sending json strings thhrough the serial port the format is as follows:

{
    "x": (x value),
    "y": (y value),
    "z": (z value),
    "w": (w value),
    "time": (u32 time in millis)
}

currently reading each line using a buffered reader which causes massive lag spikes every so often so I need to figure that out but the main goal right now is to get a functional app


# Rocket Viewer

This is the telemetry viewer for the Cascadia Engineering club model rocket project
it takes input from the serial monitor in the form of 1 line json strings and displays the data on the screen

## To Run

-Make sure you have rust installed (can be installed [here](https://rust-lang.org/))
-navigate to project folder in terminal and run it normally

``` shell
cargo run
```

## Notes

This should work on any system that currently has something printing json lines to the serial port in the following format:

{
    "x": (x value),
    "y": (y value),
    "z": (z value),
    "w": (w value),
    "time": (u32 time in millis)
}

## Issues



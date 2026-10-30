# Pipeweaver-MIDI

This program is used to control [Pipeweaver](https://github.com/pipeweaver/pipeweaver) using any MIDI device.

Each one of pipeweaver's sliders and mutes are represented and controllable with different behaviours.

## Axes

### Absolute

Absolute axes are the default. Most useful for sliders, the default config is using an input value from 0 to 127, but if
your controller is different, you can use CustomAbsolute and change the bounds.

### Relative

There are several ways relative control has been implemented on the MIDI protocol, hence this software allows you to
choose which one your controller uses. Relative mode is most useful for knobs and such.
Available modes are:

- **Binary Offset relative**: using 64 as a midpoint, anything above 64 increases the value, anything below decreases it. 
This mode is most common.
- **Two's compliment relative**: using a singned 7-bit value to declare the offset.
- **Sign Magnitude relative**: using a sign bit and a 6 bit unsigned magnitude value.

## Buttons

Buttons can be set to be toggles or push buttons.

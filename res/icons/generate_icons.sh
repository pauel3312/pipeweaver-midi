# This script requires inkscape installed to generate the various sized icons for this app, we need the following
# icon list (and location):
#    File: pipeweaver.svg, Size: 512x512 Target: /usr/share/icons/hicolor/scalable/apps/pipeweaver.svg
#    File: pipeweaver.png, Size: 48x48, Target: /usr/share/icons/hicolor/48x48/apps/pipeweaver.png
#    File: pipeweaver-large.png, Size: 128x128, Target: /usr/share/pixmaps/pipeweaver.png

inkscape pipeweaver-midi.svg --export-filename=pipeweaver-midi-large.png -w 128 -h 128
inkscape pipeweaver-midi.svg --export-filename=pipeweaver-midi.png -w 48 -h 48

{
  runCommand,
  librsvg,
  libicns,
}:

runCommand "drakeflake-macos-icons"
  {
    nativeBuildInputs = [
      librsvg
      libicns
    ];
  }
  ''
    for size in 16 32; do
      rsvg-convert -w $size -h $size ${../data/icons/drakeflake-small.svg} -o icon_$size.png
    done
    for size in 128 256 512; do
      rsvg-convert -w $size -h $size ${../data/icons/drakeflake.svg} -o icon_$size.png
    done
    mkdir -p $out/tray
    png2icns $out/drakeflake.icns icon_16.png icon_32.png icon_128.png icon_256.png icon_512.png

    # macOS only uses the alpha channel of a template image and tints it for
    # the menu bar, so the monochrome tray icons work as they are.
    for icon in drakeflake-tray drakeflake-tray-update; do
      rsvg-convert -w 44 -h 44 ${../data/icons}/$icon.svg -o $out/tray/$icon.png
    done
  ''

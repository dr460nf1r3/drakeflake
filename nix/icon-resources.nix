# On macOS Qt only looks for icon themes in `:/icons` and asks for `breeze` or `breeze-internal`.
{
  lib,
  runCommand,
  qt6,
  kdePackages,
}:

let
  folders = [
    "actions"
    "status"
    "places"
    "emblems"
    "mimetypes"
  ];
  sizes = [
    "16"
    "22"
    "32"
    "symbolic"
  ];
  icons = "${kdePackages.breeze-icons}/share/icons";

  # `themes` merged into one `breeze`: the dark theme only overrides some
  # Breeze icons, so Breeze goes in first.
  resource = name: themes: ''
    dir=$TMPDIR/${name}
    theme=$dir/icons/breeze
    mkdir -p $theme $dir/icons/breeze-internal
    for source in ${lib.escapeShellArgs themes}; do
      for folder in ${lib.escapeShellArgs folders}; do
        for size in ${lib.escapeShellArgs sizes}; do
          [ -d ${icons}/$source/$folder/$size ] || continue
          mkdir -p $theme/$folder
          cp -rL --no-preserve=mode ${icons}/$source/$folder/$size $theme/$folder/
        done
      done
    done
    sed 's/^Name=.*/Name=Breeze/' ${icons}/${lib.last themes}/index.theme \
      | grep -v '^Inherits=' > $theme/index.theme
    printf '[Icon Theme]\nName=breeze-internal\nInherits=breeze\nDirectories=\n' \
      > $dir/icons/breeze-internal/index.theme

    (
      cd $dir
      echo '<RCC><qresource prefix="/">'
      find icons -type f | sort | sed 's|.*|<file>&</file>|'
      echo '</qresource></RCC>'
    ) > $dir/icons.qrc
    ${qt6.qtbase}/libexec/rcc --binary -o $out/${name}.rcc $dir/icons.qrc
  '';
in
runCommand "drakeflake-icon-resources" { } ''
  mkdir -p $out
  ${resource "breeze" [ "breeze" ]}
  ${resource "breeze-dark" [
    "breeze"
    "breeze-dark"
  ]}
''

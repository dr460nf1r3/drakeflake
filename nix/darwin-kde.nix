# nixpkgs sets meta.platforms of KDE packages to Linux and FreeBSD only, but they build on macOS upstream.
final: prev:
let
  inherit (final) lib;
  usable = lib.filter (
    dep: !(lib.isDerivation dep) || lib.meta.availableOn final.stdenv.hostPlatform dep
  );
in
{
  kdePackages = prev.kdePackages.overrideScope (
    _kfinal: kprev: {
      mkKdeDerivation =
        args:
        (kprev.mkKdeDerivation args).overrideAttrs (old: {
          nativeBuildInputs = usable (old.nativeBuildInputs or [ ]);
          buildInputs = usable (old.buildInputs or [ ]);
          propagatedBuildInputs = usable (old.propagatedBuildInputs or [ ]);
          meta = old.meta // {
            platforms = old.meta.platforms ++ lib.platforms.darwin;
          };
        });

      # karchive 6.30 requires BZip2 and libzstd, which nixpkgs omits.
      karchive = kprev.karchive.overrideAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ [
          final.bzip2
          final.zstd
        ];
      });

      # The `usable` filter drops qtwayland on darwin, which silently drops
      # the `org.kde.guiaddons` QML module qqc2-breeze-style needs.
      kguiaddons = kprev.kguiaddons.overrideAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ [
          final.qt6.qtdeclarative
        ];
      });
    }
  );
}

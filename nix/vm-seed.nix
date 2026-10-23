{
  pkgs,
  src,
}:
let
  inherit (pkgs) lib;
  system = pkgs.stdenv.hostPlatform.system;

  pinned =
    (builtins.fromJSON (builtins.readFile (src + "/tests/fixtures/testflake/flake.lock")))
    .nodes.nixpkgs.locked;

  nixpkgsSrc = pkgs.fetchzip {
    url = "https://github.com/${pinned.owner}/${pinned.repo}/archive/${pinned.rev}.tar.gz";
    hash = pinned.narHash;
  };

  nixpkgsLib = import (nixpkgsSrc + "/lib");

  # Days since the epoch to a Gregorian date (Howard Hinnant's algorithm):
  # `lib.trivial.versionSuffix` only keeps YYYYMMDD and the short revision.
  civilDate =
    epoch:
    let
      days = epoch / 86400;
      z = days + 719468;
      era = z / 146097;
      doe = z - era * 146097;
      yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
      doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
      mp = (5 * doy + 2) / 153;
      day = doy - (153 * mp + 2) / 5 + 1;
      month = mp + 3 - 12 * (mp / 10);
      year = yoe + era * 400 + (if mp < 10 then 0 else 1);
      two = n: if n < 10 then "0${toString n}" else toString n;
    in
    "${toString year}${two month}${two day}";

  versionedLib = nixpkgsLib.extend (
    _final: prev: {
      trivial = prev.trivial // {
        versionSuffix = ".${civilDate pinned.lastModified}.${lib.substring 0 7 pinned.rev}";
        revisionWithDefault = _default: pinned.rev;
      };
    }
  );

  flakeLib = versionedLib.extend (
    final: _prev: {
      nixosSystem =
        args:
        import (nixpkgsSrc + "/nixos/lib/eval-config.nix") (
          {
            lib = final;
            system = null;
            inherit (args) modules;
          }
          // removeAttrs args [ "modules" ]
        );
      nixos = import (nixpkgsSrc + "/nixos/lib") { lib = final; };
    }
  );

  fixturePkgs = import nixpkgsSrc { inherit system; };

  evalFixture =
    name:
    flakeLib.nixosSystem {
      inherit system;
      modules = [ (src + "/tests/fixtures/${name}/configuration.nix") ];
    };

  testhostEvaled = evalFixture "testflake";

  # Without its closure in the image the offline guest build bootstraps the toolchain instead.
  # `mkBefore` matches the merge order of the generated files' nested import.
  selftesthostEvaled = testhostEvaled.extendModules {
    modules = [
      (
        { lib, pkgs, ... }:
        {
          networking.hostName = lib.mkOverride 90 "selftest-host";
          environment.systemPackages = lib.mkBefore [ pkgs.hello ];
        }
      )
    ];
  };

  manualOptions =
    (testhostEvaled.extendModules {
      modules = [
        (
          { lib, options, ... }:
          {
            documentation = {
              enable = lib.mkForce true;
            }
            // lib.optionalAttrs (options.documentation ? nixos) {
              nixos.enable = lib.mkForce true;
              nixos.includeAllModules = lib.mkForce true;
            };
          }
        )
      ];
    }).config.system.build.manual.optionsJSON;
in
{
  inherit nixpkgsSrc manualOptions;
  testhost = testhostEvaled.config.system.build.toplevel;
  selftesthost = selftesthostEvaled.config.system.build.toplevel;
  lifecyclehost = (evalFixture "lifecycleflake").config.system.build.toplevel;
  inherit (fixturePkgs)
    hello
    xterm
    cowsay
    ;
}

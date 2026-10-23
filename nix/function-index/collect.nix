# Only the XML output carries the source position of a lambda, which finds its doc comment through aliases.
{
  nixpkgs,
  system,
}:
let
  pkgs = import nixpkgs {
    inherit system;
    overlays = [ ];
    # Aliases only throw. Unfree, broken and insecure packages are still
    # attributes whose functions belong in the index.
    config = {
      allowAliases = false;
      allowUnfree = true;
      allowBroken = true;
      allowUnsupportedSystem = true;
      allowInsecurePredicate = _: true;
    };
  };
  inherit (pkgs) lib;

  nixosLib = import (nixpkgs + "/nixos/lib") { inherit lib; };
  makeDiskImage = import (nixpkgs + "/nixos/lib/make-disk-image.nix");

  attempt =
    value:
    let
      result = builtins.tryEval value;
    in
    if result.success then result.value else null;

  # A functor's position is that of the lambda its `__functor` returns.
  # Wrappers such as `makeOverridable` share one lambda: the generator drops
  # positions used by too many attributes.
  lambdaOf =
    value:
    if builtins.isFunction value then
      value
    else if builtins.isAttrs value && value ? __functor then
      attempt (value.__functor value)
    else
      null;

  isFunction = value: builtins.isFunction value || (builtins.isAttrs value && value ? __functor);

  entry = path: parent: name: {
    path = lib.concatStringsSep "." (path ++ [ name ]);
    attr = attempt (builtins.unsafeGetAttrPos name parent);
    lambda = lambdaOf (attempt parent.${name});
  };

  shallow =
    {
      path,
      set,
      keep ? _: true,
    }:
    let
      names = attempt (builtins.attrNames set);
    in
    if names == null then
      [ ]
    else
      map (entry path set) (builtins.filter (name: keep name && isFunction (attempt set.${name})) names);

  deep =
    {
      path,
      set,
      depth ? 4,
      skip ? [ ],
    }:
    let
      walk =
        level: prefix: current:
        lib.concatMap (
          name:
          let
            value = attempt current.${name};
            here = prefix ++ [ name ];
          in
          if builtins.elem (lib.concatStringsSep "." here) skip then
            [ ]
          else if isFunction value then
            [ (entry prefix current name) ]
          else if
            level < depth
            && builtins.isAttrs value
            && !(lib.isDerivation value)
            && !(lib.isOptionType value)
            && !(value ? _type)
          then
            walk (level + 1) here value
          else
            [ ]
        ) (builtins.attrNames current);
    in
    walk 1 path set;

  isBuilder = name: builtins.match "(build|mk|fetch|make|wrap|write|run|with)[A-Z].*" name != null;
in
lib.concatLists [
  (deep {
    path = [ "lib" ];
    set = lib;
    skip = [
      "lib.maintainers"
      "lib.teams"
      "lib.licenses"
      "lib.systems.examples"
      "lib.systems.parse"
      "lib.systems.architectures"
      "lib.platforms"
      # Copies under another name: `types.types` is `types` again, and the
      # merge functions are lib-internal tables.
      "lib.types.types"
      "lib.mergeAttrBy"
      "lib.misc.mergeAttrBy"
    ];
  })
  (deep {
    path = [
      "nixos"
      "lib"
    ];
    set = nixosLib;
    depth = 2;
  })
  (shallow {
    path = [ "pkgs" ];
    set = pkgs;
  })
  (shallow {
    path = [
      "pkgs"
      "stdenv"
    ];
    set = pkgs.stdenv;
  })
  (shallow {
    path = [
      "pkgs"
      "formats"
    ];
    set = pkgs.formats;
  })
  (deep {
    path = [
      "pkgs"
      "writers"
    ];
    set = pkgs.writers;
    depth = 1;
  })
  (deep {
    path = [
      "pkgs"
      "testers"
    ];
    set = pkgs.testers;
    depth = 2;
  })
  (shallow {
    path = [
      "pkgs"
      "dockerTools"
    ];
    set = pkgs.dockerTools;
  })
  (shallow {
    path = [
      "pkgs"
      "ociTools"
    ];
    set = pkgs.ociTools;
  })
  (shallow {
    path = [
      "pkgs"
      "vmTools"
    ];
    set = pkgs.vmTools;
  })
  (shallow {
    path = [
      "pkgs"
      "releaseTools"
    ];
    set = pkgs.releaseTools;
  })
  (shallow {
    path = [
      "pkgs"
      "appimageTools"
    ];
    set = pkgs.appimageTools;
  })
  (shallow {
    path = [
      "pkgs"
      "rustPlatform"
    ];
    set = pkgs.rustPlatform;
  })
  (shallow {
    path = [
      "pkgs"
      "haskell"
      "lib"
    ];
    set = pkgs.haskell.lib;
  })
  (shallow {
    path = [
      "pkgs"
      "python3Packages"
    ];
    set = pkgs.python3Packages;
    keep = isBuilder;
  })
  (shallow {
    path = [ ];
    set = {
      make-disk-image = makeDiskImage;
    };
  })
]

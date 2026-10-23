shard: skip: sys:
let
  pkgs = sys.pkgs;
  lib = pkgs.lib;
  __SHARD__
  skipSet = builtins.listToAttrs (map (n: { name = n; value = true; }) skip);
  isSet = n:
    let r = builtins.tryEval (
      let v = pkgs.${n}; in
      builtins.isAttrs v && (v.type or null) != "derivation" && (v.recurseForDerivations or false));
    in r.success && r.value;
in builtins.listToAttrs
  (map (n: { name = n; value = isSet n; })
    (builtins.filter (n: !(skipSet ? ${n}) && inShard n) (builtins.attrNames pkgs)))

shard: skip: sys:
let
  pkgs = sys.pkgs;
  lib = pkgs.lib;
  __SHARD__
  skipSet = builtins.listToAttrs (map (n: { name = n; value = true; }) skip);
  one = n:
    let r = builtins.tryEval (
      let p = pkgs.${n}; in
      if builtins.isAttrs p && (p.type or null) == "derivation" then
        let
          d = p.meta.description or "";
          v = p.version or "";
          clean = s: if builtins.isString s then builtins.unsafeDiscardStringContext s else "";
          e = { d = clean d; v = clean v; };
        in builtins.deepSeq e e
      else null);
    in if r.success then r.value else null;
in builtins.listToAttrs
  (map (n: { name = n; value = one n; })
    (builtins.filter (n: !(skipSet ? ${n}) && inShard n) (builtins.attrNames pkgs)))

shard: skip: sys:
let
  pkgs = sys.pkgs;
  lib = pkgs.lib;
  __SHARD__
  skipSet = builtins.listToAttrs (map (n: { name = n; value = true; }) skip);
  clean = s: if builtins.isString s then builtins.unsafeDiscardStringContext s else "";
  entry = p: { d = clean (p.meta.description or ""); v = clean (p.version or ""); };
  walk = prefix: depth: attrs:
    builtins.concatMap (n:
      let
        full = prefix + n;
        r = builtins.tryEval (
          let
            p = attrs.${n};
            found =
              if builtins.isAttrs p && (p.type or null) == "derivation" then [ { name = full; value = entry p; } ]
              else if depth < 2 && builtins.isAttrs p && (p.recurseForDerivations or false) then walk (full + ".") (depth + 1) p
              else [ ];
          in builtins.deepSeq found found);
      in if r.success then r.value else [ ])
      (builtins.filter (n: !(skipSet ? ${n}) && !(skipSet ? ${prefix + n}) && (depth > 0 || inShard n))
        (builtins.attrNames attrs));
in builtins.listToAttrs (walk "__SET__." 0 (pkgs.__SET__ or { }))

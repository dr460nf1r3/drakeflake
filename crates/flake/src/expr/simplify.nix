  lib = sys.pkgs.lib;
  simplify = depth: v:
    let t = builtins.typeOf v; in
    if t == "bool" || t == "int" || t == "float" || t == "null" then v
    else if t == "string" then builtins.unsafeDiscardStringContext v
    else if t == "path" then toString v
    else if t == "list" then
      (if depth > 0
       then map (simplify (depth - 1)) (lib.take 200 v)
            ++ lib.optional (builtins.length v > 200) { _drakeflakeOpaque = "more items"; }
       else { _drakeflakeOpaque = "list"; })
    else if t == "set" then
      (if (v.type or null) == "derivation" || v ? outPath then { _drakeflakeDrv = v.name or "derivation"; }
       else if depth > 0 then builtins.mapAttrs (_: simplify (depth - 1)) v
       else { _drakeflakeOpaque = "attribute set"; })
    else { _drakeflakeOpaque = t; };
  safe = v:
    let s = simplify 3 v; r = builtins.tryEval (builtins.deepSeq s s); in
    if r.success then r.value else { _drakeflakeError = true; };
  cfgRoot = __CONFIG__;
  valueAt = loc:
    let
      root = if lib.take 2 loc == [ "home-manager" "users" ] then sys.config else cfgRoot;
      r = builtins.tryEval (lib.attrByPath loc null root);
    in
    if r.success then safe r.value else { _drakeflakeError = true; };

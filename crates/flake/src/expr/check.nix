loc: sys:
let
  pkgs = sys.pkgs;
  lib = pkgs.lib;
  config = sys.config;
  hmLoc =
    if builtins.length loc > 3 && builtins.elemAt loc 0 == "home-manager" && builtins.elemAt loc 1 == "users"
    then lib.drop 3 loc else null;
  hmTree = user:
    let u = sys.options.home-manager.users; in
    if u ? valueMeta.attrs.${user}.configuration.options
    then u.valueMeta.attrs.${user}.configuration.options
    else u.type.getSubOptions [ ];
  opt =
    if hmLoc != null && sys.options ? home-manager && sys.options.home-manager ? users
    then lib.attrByPath hmLoc null (hmTree (builtins.elemAt loc 2))
    else lib.attrByPath loc null sys.options;
  elemOf = t: t.nestedTypes.elemType or null;
  deep = t: v:
    let name = t.name or ""; e = elemOf t; in
    t.check v && (
      if e == null then true
      else if name == "listOf" then builtins.all (deep e) v
      else if name == "attrsOf" || name == "lazyAttrsOf" then builtins.all (deep e) (builtins.attrValues v)
      else if name == "nullOr" then v == null || deep e v
      else true);
  value = (__VALUE__);
  result = builtins.tryEval (deep opt.type value);
in
  if !(lib.isOption opt) then { result = "unknown"; }
  else if result.success then {
    result = if result.value then "ok" else "invalid";
    type = opt.type.description;
  }
  else { result = "error"; type = opt.type.description; }

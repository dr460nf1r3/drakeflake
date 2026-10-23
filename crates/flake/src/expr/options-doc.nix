shard: skip: sys:
let
  lib = sys.pkgs.lib;
  __WALK__
  nixpkgsRoot = toString sys.pkgs.path;
  guard = v: let r = builtins.tryEval (builtins.deepSeq v v); in if r.success then r.value else null;
  text = v: if builtins.isAttrs v && v ? text then v.text else v;
  render = v: if builtins.isAttrs v && v ? _type && v ? text then v else lib.options.renderOptionValue v;
  docOf = o: lib.filterAttrs (_: x: x != null) {
    _drakeflakeOption = lib.showOption o.loc;
    type = guard (o.type.description or "unspecified");
    description = guard (text (o.description or ""));
    default = guard (if o ? defaultText then render o.defaultText else if __DEFAULTS__ && o ? default then render o.default else null);
    example = guard (if o ? example then render o.example else null);
    declarations = guard (map (d: lib.removePrefix (nixpkgsRoot + "/") (toString d)) (o.declarations or [ ]));
    readOnly = guard (o.readOnly or false);
  };
  subOptionsOf = o:
    let r = builtins.tryEval (
      if (o.visible or true) == "shallow" then { }
      else if o.type ? getSubOptions then o.type.getSubOptions o.loc
      else { });
    in if r.success && builtins.isAttrs r.value then r.value else { };
  # "~name" in skip: the docs of that option broke once, so emit only safe fields.
  bareSet = builtins.listToAttrs (map (n: { name = lib.removePrefix "~" n; value = true; })
    (builtins.filter (lib.hasPrefix "~") skip));
  bareDoc = o: {
    _drakeflakeOption = lib.showOption o.loc;
    type = guard (o.type.description or "unspecified");
    description = "(This option's documentation could not be evaluated.)";
  };
  leaf = key: o:
    if hidden o then null
    else if bareSet ? ${key} then bareDoc o
    else docOf o // { _drakeflakeSub = walk 2 key (subOptionsOf o); };
in walk 0 "" __TREE__

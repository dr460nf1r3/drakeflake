sys:
let
  nameOf = p:
    let r = builtins.tryEval (
      let n = p.pname or (builtins.parseDrvName (p.name or "")).name; in
      if builtins.isString n then builtins.unsafeDiscardStringContext n else null);
    in if r.success then r.value else null;
  all = builtins.tryEval (map nameOf sys.config.environment.systemPackages);
in if all.success then builtins.filter (n: n != null) all.value else [ ]

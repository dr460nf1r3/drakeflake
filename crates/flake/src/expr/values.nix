wanted: sys:
let
  __SIMPLIFY__
in builtins.listToAttrs (map (w: { name = w.n; value = valueAt w.l; }) wanted)

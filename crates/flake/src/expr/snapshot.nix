shard: skip: sys:
let
  __SIMPLIFY__
  __WALK__
  __USER_FILES__
  leaf = key: o:
    if hidden o then null
    else { _drakeflakeOption = lib.showOption o.loc; } // entryOf o;
in walk 0 "" __TREE__

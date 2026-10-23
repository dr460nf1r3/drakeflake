_shard: skip: sys:
let
  __SIMPLIFY__
  __USER_FILES__
  skipSet = builtins.listToAttrs (map (n: { name = n; value = true; }) skip);
  tree = __TREE__;
  ownerOf = node: loc:
    if lib.isOption node then node
    else if loc == [ ] || !(builtins.isAttrs node) || !(node ? ${builtins.head loc}) then null
    else ownerOf node.${builtins.head loc} (builtins.tail loc);
  owner = l: let r = builtins.tryEval (ownerOf tree l); in if r.success then r.value else null;
  found = builtins.filter (o: o != null) (map (wanted: owner wanted.l) __NAMES__);
  named = map (o: { name = lib.showOption o.loc; option = o; }) found;
in builtins.listToAttrs (map (e: { name = e.name; value = entryOf e.option; })
  (builtins.filter (e: !(skipSet ? ${e.name})) named))

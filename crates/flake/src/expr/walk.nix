  __SHARD__
  skipSet = builtins.listToAttrs (map (n: { name = n; value = true; }) skip);
  walk = depth: path: node:
    builtins.listToAttrs (builtins.concatMap (n:
      let key = if path == "" then n else path + "." + n; in
      if skipSet ? ${key} || lib.hasPrefix "_" n || (depth == 1 && !(inShard key)) then [ ]
      else [ { name = key; value = visit depth key node.${n}; } ])
      (builtins.attrNames node));
  visit = depth: key: v:
    if lib.isOption v then (if depth == 0 && shard.index != 0 then null else leaf key v)
    else if builtins.isAttrs v && depth < 16 then walk (depth + 1) key v
    else null;
  hidden = o: (o.internal or false) || (o.visible or true) == false;

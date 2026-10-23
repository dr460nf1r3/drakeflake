attr: sys:
let
  pkgs = sys.pkgs;
  lib = pkgs.lib;
  found = builtins.tryEval (lib.attrByPath (lib.splitString "." attr) null pkgs);
  p = if found.success then found.value else null;
  meta = p.meta or { };
  guard = v: let r = builtins.tryEval (builtins.deepSeq v v); in if r.success then r.value else null;
  str = v: if builtins.isString v then builtins.unsafeDiscardStringContext v else null;
  licenseName = l: if builtins.isAttrs l then str (l.spdxId or l.fullName or l.shortName or null) else str l;
  licenses = lib.toList (meta.license or [ ]);
  homepages = lib.toList (meta.homepage or [ ]);
in
  if !(builtins.isAttrs p) then null
  else lib.filterAttrs (_: v: v != null) {
    name = guard (str (p.name or null));
    version = guard (str (p.version or null));
    description = guard (str (meta.description or null));
    longDescription = guard (str (meta.longDescription or null));
    homepage = guard (if homepages == [ ] then null else str (builtins.head homepages));
    licenses = guard (builtins.filter (l: l != null) (map licenseName licenses));
    unfree = guard (builtins.any (l: builtins.isAttrs l && !(l.free or true)) licenses);
    mainProgram = guard (str (meta.mainProgram or null));
    maintainers = guard (map (m: str (m.name or m.github or null)) (meta.maintainers or [ ]));
    position = guard (if meta ? position then lib.removePrefix (toString pkgs.path + "/") (str meta.position) else null);
    broken = guard (meta.broken or false);
    outputs = guard (p.outputs or null);
  }

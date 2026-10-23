  nixpkgsRoot = toString sys.pkgs.path;
  frameworkRoots = __FRAMEWORK_ROOTS__;
  isUserFile = f:
    builtins.isString f && lib.hasPrefix "/" f
    && (!(lib.hasPrefix nixpkgsRoot f) || f == nixpkgsRoot + "/flake.nix")
    && !(builtins.any (root: lib.hasPrefix root f) frameworkRoots);
  filesOf = o:
    let r = builtins.tryEval (builtins.filter isUserFile (map toString (o.files or [ ]))); in
    if r.success then builtins.deepSeq r.value r.value else [ ];
  # Evaluating every value took minutes on real systems and broken defaults forced retries,
  # so only values the user defines are evaluated here.
  entryOf = o:
    let u = filesOf o; in
    { u = u; } // lib.optionalAttrs (u != [ ]) { v = valueAt o.loc; };

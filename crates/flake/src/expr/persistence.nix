sys:
let
  inherit (builtins)
    attrNames
    attrValues
    concatLists
    concatMap
    filter
    isAttrs
    isList
    isString
    mapAttrs
    tryEval
    ;

  cfg = sys.config;
  opts = sys.options;
  hmUsers = cfg.home-manager.users or { };

  rows =
    storage: scope: user: set:
    map (dir: {
      kind = "dir";
      path = dir.dirPath;
      entry = dir.directory;
      inherit storage scope user;
    }) (set.directories or [ ])
    ++ map (file: {
      kind = "file";
      path = file.filePath;
      entry = file.file;
      inherit storage scope user;
    }) (set.files or [ ]);

  enabled = set: set.enable or true;
  perAttr = fn: set: concatLists (attrValues (mapAttrs fn set));

  system = perAttr (
    storage: set:
    if enabled set then
      rows storage "system" "" set ++ perAttr (user: rows storage "user" user) (set.users or { })
    else
      [ ]
  ) cfg.environment.persistence;

  hasHomeModule = hm: hm ? home && hm.home ? persistence;

  home = perAttr (
    user: hm:
    if hasHomeModule hm then
      perAttr (storage: set: if enabled set then rows storage "home" user set else [ ]) hm.home.persistence
    else
      [ ]
  ) hmUsers;

  plain = value: isAttrs value && !(value ? _type);
  entryName =
    entry:
    if isString entry then
      entry
    else if plain entry && isString (entry.directory or entry.file or null) then
      entry.directory or entry.file
    else
      null;
  listed =
    set: kind:
    let
      list = set.${kind} or [ ];
    in
    if isList list then filter (name: name != null) (map entryName list) else [ ];
  namesOf =
    scope: user: storage: set:
    if plain set then
      map (entry: {
        inherit scope user storage entry;
      }) (listed set "directories" ++ listed set "files")
    else
      [ ];
  definedIn =
    def:
    let
      found = tryEval (
        if plain def.value then
          perAttr (
            storage: set:
            namesOf "system" "" storage set
            ++ (
              if plain set && plain (set.users or { }) then
                perAttr (user: namesOf "user" user storage) (set.users or { })
              else
                [ ]
            )
          ) def.value
        else
          [ ]
      );
    in
    if found.success then map (row: row // { inherit (def) file; }) found.value else [ ];
in
if opts ? environment && opts.environment ? persistence then
  {
    entries = system ++ home;
    storages = attrNames cfg.environment.persistence;
    homes = mapAttrs (user: _: cfg.users.users.${user}.home or "/home/${user}") hmUsers;
    homeModule = mapAttrs (_: hasHomeModule) hmUsers;
    sources = concatMap definedIn (opts.environment.persistence.definitionsWithLocations or [ ]);
  }
else
  null

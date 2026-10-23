  hexValue = {
    "0" = 0; "1" = 1; "2" = 2; "3" = 3; "4" = 4; "5" = 5; "6" = 6; "7" = 7;
    "8" = 8; "9" = 9; "a" = 10; "b" = 11; "c" = 12; "d" = 13; "e" = 14; "f" = 15;
  };
  bucket = key:
    let h = builtins.hashString "md5" key; in
    hexValue.${builtins.substring 0 1 h} * 16 + hexValue.${builtins.substring 1 1 h};
  inShard = key: shard.count <= 1 || lib.mod (bucket key) shard.count == shard.index;

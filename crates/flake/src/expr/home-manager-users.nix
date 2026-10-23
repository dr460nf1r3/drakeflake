sys:
if sys.options ? home-manager && sys.options.home-manager ? users then
  builtins.attrNames (sys.config.home-manager.users or { })
else
  null

.pragma library

// Model status names the QML side matches on. Rust owns the values
// (validate types, package search, change checks): compare against these.
var editorBool = "bool";
var editorInt = "int";
var editorFloat = "float";
var editorString = "string";
var editorText = "text";
var editorPath = "path";
var editorEnum = "enum";
var editorList = "list";
var editorPackage = "package";
var editorRaw = "raw";

var commandSearching = "searching";
var commandDownloading = "downloading";
var commandMissingIndex = "missing-index";
var commandError = "error";

var checkPending = "pending";
var checkOk = "ok";
var checkInvalid = "invalid";
var checkFailed = "failed";
var checkUnknown = "unknown";

var outcomeOk = "ok";

var importIdle = "";
var importChecking = "checking";
var importImported = "imported";
var importMissing = "missing";
var importFailed = "failed";

var valueLoading = "loading";
var valueReady = "ready";
var valueError = "error";
var valueUnknown = "unknown";

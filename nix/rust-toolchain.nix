# The newest nightly that has every component, as some nightlies lack clippy or rust-analyzer.
{ rust-bin }:
rust-bin.selectLatestNightlyWith (
  toolchain:
  toolchain.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
      "llvm-tools"
    ];
  }
)

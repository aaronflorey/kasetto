class Sukiro < Formula
  desc "Fast binary CLI to sync AI skills from local and GitHub sources"
  homepage "https://github.com/pivoshenko/sukiro"
  url "https://github.com/pivoshenko/sukiro/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_RELEASE_SHA256"
  license "MIT"

  depends_on "go" => :build

  def install
    system "go", "build", *std_go_args(ldflags: "-s -w"), "./cmd/sukiro"
  end

  test do
    assert_match "sukiro", shell_output("#{bin}/sukiro --help 2>&1", 0)
  end
end

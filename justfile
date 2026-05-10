format-rs:
  cargo fmt

format-site:
  cd site && pnpm format

format: format-rs format-site

lint-rs:
  cargo clippy -- -D warnings

lint-site:
  cd site && pnpm lint

lint: lint-rs lint-site

test:
  cargo test

update-rs:
  cargo update

update-site:
  cd site && pnpm update

update: update-rs update-site

build-rs:
  cargo build --release

build-site:
  cd site && pnpm build

build: build-rs build-site

serve-site:
  cd site && pnpm dev

changelog:
  git-cliff --output CHANGELOG.md

check: format lint test build

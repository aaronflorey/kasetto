format-rs:
  cargo fmt

format-next:
  cd landing && pnpm format

format: format-rs format-next

lint-rs:
  cargo clippy -- -D warnings

lint-next:
  cd landing && pnpm lint

lint: lint-rs lint-next

test:
  cargo test

update:
  cargo update

build-rs:
  cargo build --release

build-next:
  cd landing && pnpm build

build-docs:
  mkdocs build

build: build-rs build-next

serve-docs:
  mkdocs serve

serve-landing:
  cd landing && pnpm dev

changelog:
  git-cliff --output CHANGELOG.md

check: format lint test build

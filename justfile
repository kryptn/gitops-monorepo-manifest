default:
  @just --list

release part="patch":
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo bump {{part}}
    version=$(cargo get version --pretty)
    git add -u
    git commit -m "bump version to $version"
    git tag $version

push-release:
    git push
    git push --tags

check:
    cargo check

fmt:
    cargo fmt

test:
    cargo test

clippy:
    cargo clippy

ci:
    just check
    just fmt
    just test
    just clippy

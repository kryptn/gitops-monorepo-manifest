default: (release "patch")

release part:
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
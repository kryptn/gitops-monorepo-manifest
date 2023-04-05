# Manifest

### A way to determine what to do with a monorepo

This tool does two primary things:

- With a given target, determine if it's been changed relative to where it was branched
- Get a git sha from the target
    - if that target has been changed, it should be the HEAD of the current branch
    - if that target hasn't changed, it should be the merge-base ref


## Usage

```
❯ manifest derive --help
Usage: manifest derive [OPTIONS]

Options:
      --config <CONFIG>  [default: .manifest.yaml]
      --head <HEAD>      Defaults to the current branch
      --base <BASE>      Defaults to the base defined in the manifest config
  -f, --force...         Forces all services to be activated
      --force-on-base    Apply force if building on base
      --actions-output   Write manifest into github actions output
      --step-summary     Write manifest summary into github actions step summary
  -h, --help             Print help
```



### The Manifest file

```yaml
base: main
targets:
  apollo:
    path: services/apollo/*
    activated_by:
      - common-dependency

  artemis:
    path: services/artemis/*
    activated_by:
      - common-dependency

  documentation:
    path: documentation/*
    globs:
      - services/apollo/docs/*
      - services/artemis/docs/*

  common-dependency:
    path: packages/common/*
    resistance: 1
```

| spec | description |
| ---- | ----------- |
`.base` | the branch every branch will be compared against
`.targets[]` | the name of the target
`.targets[].path` | the path for the target
`.targets[].resistance` | when forced, it takes this many flags to activate it
`.targets[].globs[].` | an additional list of paths that will activate the target
`.targets[].activated_by[].` | a list of targets that will activate this target when activated

## Github Actions

Built with github actions in mind, there's a couple flags that'll write to files provided by github actions.

`--actions-output` will append three outputs to the file `$GITHUB_OUTPUT`

- `manifest`, a encoded json object with derived manifest output
- `changed_targets`, an encoded json list of targets that have changed
- `unchanged_targets`, an encoded json list of targets that have not changed

`--step-summary` will write a pretty-printed json manifest into the file at `$GITHUB_STEP_SUMMARY` and will show on the summary page for your github action.
# Contributing

This is a guide for anyone that is looking to contribute to `faucet`.
This document outlines basics such as commit messages and pull request
structure.

## Commit Messages

In the `faucet` repository we follow [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

Conventional Commits allows us to keep track of what commits are about
and make it easier for version control systems to automatically generate
Change-Logs and other cool things.

We highly recommend you read through the Conventional Commits specification
but we will leave some examples below for a quick start.

### Examples

#### A new feature

Let's say you have implemented a new feature. Thank you for taking the
time to improve `faucet`. This feature allows the users to run Shiny Apps
but with a twist. You will probably want to commit these changes.

A conventional commit for said feature might look something like this:

```
feat(shiny): Allows the user to run shiny apps with a twist
```

You may include other information in the commit but it is not
strictly necessary.


### Fixing a bug

If you fixed a bug, it should start with the `fix` prefix.

```
fix(windows): Fixes weird Windows behaviour
```

### Documentation

All commits for documentation must have the `docs` prefix.
Since documentation may be written by stages it is highly
encouraged to squash commits when ever possible

```
docs(windows): Documents how to deal with Windows things
```

### Other tasks

Other tasks not related to features, fixes or documentation will likely
fall under the `chore` prefix.

The `chore` prefix typically involves things like dependency updates,
versions bumps, deleting unused code, etc.

```
chore: Updates dependencies
```

## Pull request structure

Every pull request should ideally close one issue. We do not ask for
extensive information on pull requests, but please include an issue
where what is being done is described.

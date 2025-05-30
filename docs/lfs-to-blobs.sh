#!/usr/bin/env bash
# Embedding screenshots from Git LFS in our documentation via
# https://media.githubusercontent.com/ depletes our GitHub LFS quota
# so we instead convert our LFS objects into regular Git blobs
# in the branch configured below on release so that we can embed them
# via https://raw.githubusercontent.com/ which has no quota.

BLOB_BRANCH=screenshots
TAG=$1

set -e

if [ -z "$TAG" ]; then
    echo "usage: $0 <tag>"
    exit 1
fi


if git show-ref --tags --quiet --verify refs/tags/$TAG; then
    git switch --detach $TAG
else
    echo "error: $TAG is not a tag."
    exit 1
fi

LFS_FILES=$(git lfs ls-files --name-only)
if [ -z "$LFS_FILES" ]; then
    echo "error: no LFS files found"
    git switch -
    exit 1
fi


WORKTREE_PATH=$(git rev-parse --show-toplevel)/.git/lfs-to-blob-worktree
NEW_BRANCH="$BLOB_BRANCH-$TAG"

git worktree add "$WORKTREE_PATH" "$BLOB_BRANCH" -b "$NEW_BRANCH"
mkdir "$WORKTREE_PATH/$TAG"
echo "$LFS_FILES" | xargs -I {} cp --parents "{}" "$WORKTREE_PATH/$TAG"
git -C "$WORKTREE_PATH" add "$TAG"
git -C "$WORKTREE_PATH" commit -m "add $TAG"
git worktree remove "$WORKTREE_PATH"
git switch -
echo "created branch $NEW_BRANCH"

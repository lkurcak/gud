# Smart branch tracking

The owner's usual git workflow:

1. create a feature branch
2. create a pull request
3. squash-merge the pull request into main
4. the feature branch is (automatically) deleted from origin

The local branch stays on the dev machine. Deleting it by hand means recalling which branches
those were and deciding whether each is safe to delete, because `git branch -d` often doesn't
see a squash-merged branch as merged.

`gud branch` could highlight the branches that used to track a branch on origin that was later
(squash-)merged and deleted.

Git may already say this: after a fetch with `--prune`, `git branch -vv` shows such a branch's
upstream as `gone`, and `git for-each-ref --format='%(refname:short) %(upstream:track)'
refs/heads` prints `[gone]` for it.

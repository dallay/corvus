### [Renovate](https://docs.renovatebot.com/configuration-options)

#### Validate config

validate renovate.json via [renovate-config-validator](https://docs.renovatebot.com/config-validation/)

```shell
npx --yes --package renovate -- renovate-config-validator
```

> output:
>
> ```
> INFO: Validating renovate.json
> INFO: Config validated successfully
> ```

#### Validate PRs

Create a new Git branch that matches the {{branchPrefix}}reconfigure pattern. For example, if you're using the default
prefix renovate/, your branch name must be renovate/reconfigure.
see [Validation of Renovate config change PRs](https://docs.renovatebot.com/config-validation/#validation-of-renovate-config-change-prs)

```shell
git checkout -b renovate/reconfigure
```

### No rebasing if you have made edits

If you push a new commit to a Renovate branch, for example to fix your code so the tests pass, then Renovate stops all
updates of that branch. It is up to you to either finish the job and merge the PR, or rename it and close it so that
Renovate can take over again.
see [no-rebasing-if-you-have-made-edits](https://docs.renovatebot.com/updating-rebasing/#no-rebasing-if-you-have-made-edits)

So, if we want to keep the bot running, we should only consider pushing additional code when preparing to merge PRs.

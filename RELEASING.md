In order to [release a new version](https://github.com/ExpediaGroup/mockql-rs/releases) we need to draft a new release
and tag the commit. Releases follow [semantic versioning](https://semver.org/) and specify major, minor and patch version.

Once a release is published it will trigger the corresponding [Github Action](https://github.com/ExpediaGroup/mockql-rs/blob/main/.github/workflows/release.yml)
based on the published release event. The release workflow will then proceed to build the binary and upload it as a release asset.

### Release requirements

- tag should specify the newly released version following [semantic versioning](https://semver.org/)
- tag and release name should match
- release should contain information about all the change sets included in the given release. We are using `release-drafter` to help automatically
  collect this information and generate automatic release notes.

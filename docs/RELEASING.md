# Releasing Quboid

Pushing an `X.Y.Z` tag runs [`.github/workflows/release.yml`](../.github/workflows/release.yml), which refuses to go
on unless the tag matches the version in [`Cargo.toml`](../Cargo.toml), then publishes the MSI,
the portable archive and the checksums as a GitHub release and submits the MSIX
to the Microsoft Store. Tags carry no `v` prefix, so the tag and the version in
`Cargo.toml` are the same string.

Every version needs a `## [X.Y.Z]` section in both [`CHANGELOG.md`](../CHANGELOG.md)
and [`CHANGELOG.it.md`](../CHANGELOG.it.md), written for the people who use Quboid.
The English and Italian sections become the notes of the GitHub release, and
their plain-text form becomes the "What's new in this version" of the matching
Store listing: listings in Italian get the Italian text, every other listing
the English one. The Store caps that text at 1500 characters. CI checks both
sections on every pull request and the release workflow checks them again
before building, so a version bump without its changelog cannot be tagged:

```powershell
# Check the version in Cargo.toml, as CI does.
.\scripts\release-notes.ps1 -Check

# Preview what the Store listing will say.
.\scripts\release-notes.ps1 -Language it -Format Store
```

To put the notes in, the Store job uploads the package into a draft
submission, writes the release notes and the English/Italian listing copy
(`shortDescription`, `description`, `features` and `keywords`) into its
listings with `scripts\set-store-release-notes.ps1`, and only then commits it.

Running the workflow by hand from the Actions tab rehearses all of that without
publishing anything: it builds, packages and verifies, and stops short of the
release and the submission (unless `update_store_listing` is checked to refresh
the Store listing copy without uploading a new MSIX). Nothing creates a tag, so
there is no way to spend a version by accident.

`Cargo.toml` is the only place a version is written. The MSIX identity derives
from it as `X.Y.Z.0`: the fourth part belongs to the Store and stays zero, which
is why a prerelease tag such as `1.0.0-rc.1` produces a GitHub release but no
Store submission. The Store also rejects a package that does not outrank the
published one, so a submission that fails certification cannot be retried under
the same version; the fix ships as a new patch release.

The submission needs five repository secrets — `STORE_PRODUCT_ID`,
`STORE_TENANT_ID`, `STORE_SELLER_ID`, `STORE_CLIENT_ID` and
`STORE_CLIENT_SECRET` — obtained from Partner Center and a Microsoft Entra ID
application holding the Manager role on the account. With none of them set the
job reports a skip instead of failing, so a fork releases without them. Setting
only some of them fails the release instead: a secret that exists but is empty
counts as missing, which is the likeliest way to break a release that worked
before. Partner Center still owns what no API can create: the reserved name, the
listing and the first submission are done by hand once, and every tag after that
ships on its own. Certification itself is asynchronous, so the job waits on the
outcome and fails if Microsoft rejects the package.

`STORE_CLIENT_SECRET` expires on the date chosen when it was created in Entra,
and releases fail to authenticate from that day on. Issuing a new secret on the
same application and updating the repository secret is the whole fix; nothing
else needs to change.

Releases are immutable, so deleting one does not free its tag name: the name is
burned and that version can never be released again. A release published by
mistake is superseded by the next patch rather than replaced.

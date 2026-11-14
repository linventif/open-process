# Data and storage

This page describes the storage and network behavior implemented in the
repository. It is a technical guide, not a statement about the retention settings
of the production hosting provider.

## Documents and sessions

Models and analyses are handled by the application on your device. Automatic
saving keeps document content, names, geometry, styles, source and exercise
answers locally. Closing a tab retains its last automatic copy; **Saved diagrams**
on the home screen reopens it.

- **Desktop:** the application creates **Open Process** inside the OS Documents
  folder, including redirected Documents locations. Linux uses `~/Documents`
  when no Documents directory is configured. Each document gets a stable UUID
  filename (`<id>.opx`) and a small `<id>.json` metadata file. Renaming a document
  retains its identity, and documents sharing a name keep separate copies.
- **Browser:** each automatic document is a local-storage record under
  `open-process-document-<id>`, belonging to the app's origin and browser profile.
  The open workspace references confirmed records without duplicating their
  entire payloads.

Changed documents are saved about once per second while the app runs. Native
file writes use one background writer and atomic replacement; closing a document
or exiting finishes pending writes. Browser saves also flush on page hiding and
normal unload events. Abrupt process termination or power loss can interrupt
changes not yet confirmed. Normal automatic saving is silent; storage errors
remain visible and unsaved nonempty documents retain close confirmation. An
untouched empty draft creates no library file; an existing diagram cleared to
empty updates its automatic copy.

**Save/Save as** creates or updates a user-chosen native file, or downloads a
browser copy. Automatic saving writes the managed copy independently of that
manual destination; opening/importing a file does not automatically overwrite
its source. The current document paths do not upload models to an application
server. Desktop writes stage and sync before replacement. Browser downloads
report that a download was started; a successful automatic local-storage write
can independently confirm a local saved copy. See the
[save outcomes and import limits](documents-et-compatibilite.md#import-safety-and-save-outcomes).

Session restoration stores open-document references, pending unsaved payloads,
view state and layout through eframe's persistence layer. Desktop sessions use
application storage; browser sessions use local storage. Language settings and
cached previews are also persisted. Confirmed automatic copies are acknowledged
separately from the manual destination and from in-flight edits.

There is no account-based document sync in these storage paths. Another browser,
profile, device or site origin has its own session. Private browsing, storage
limits or clearing site data can remove the browser library and session. Local
storage has a browser-defined quota shared with session/preferences/previews;
quota or permission failures are reported instead of marking the write saved.
Stale document revisions are rejected rather than silently replacing a newer
copy; simultaneous applications are not a collaborative editing system. Keep separate `.opx`
files for backups. The [compatibility guide](documents-et-compatibilite.md)
documents the file and session schemas.

## Remove local data

Export any documents you want to keep first.

- **Browser:** close the app's tabs, then use the browser's site settings to clear
  stored data for the origin serving Open Process. This removes the restored
  workspace, preferences and cached previews; it may also remove other data for
  the same origin. Closing tabs first prevents an open app from saving again.
- **Desktop:** quit the application before removing its eframe application
  storage for Open Process. The location depends on the operating system and
  eframe version. If you need help finding it, provide your OS and app release
  through [support](../SUPPORT.md); do not delete unrelated application folders.
- **Desktop automatic library:** quit the application and remove the managed
  `.opx` and matching `.json` files from **Documents/Open Process**, or remove that
  folder to clear the library. The app recreates the folder on its next launch.
- **Saved files:** `.opx` files, exports and downloaded copies are separate.
  Remove those through your file manager if you no longer want them.

Closing a document keeps its automatic copy. Removing a manual/downloaded file
does not delete its automatic copy or every persisted session/preview, including
legacy session data.

## Website and external services

The public site and browser app are served through the hosting setup described
in [website/README.md](../website/README.md). Loading them sends ordinary web
requests to the host. The presentation website also requests the latest release
metadata from the public GitHub API to populate download links. Following GitHub
links or downloading a release sends requests to GitHub and its delivery services.
These release-metadata requests do not include diagram content.

The checked-in application and website sources do not configure an analytics or
telemetry SDK. This does not establish whether hosting-level logs or analytics
are enabled: production settings and provider retention have not been audited
as part of this documentation. Fonts used by the app and site are bundled locally.

## Sharing reports

Issues, pull requests and their attachments are public. Use synthetic or
anonymised models and remove personal information, business data, credentials
and local paths from screenshots and logs before sharing. For private matters,
see [SUPPORT.md](../SUPPORT.md); for vulnerabilities, see
[SECURITY.md](../SECURITY.md).

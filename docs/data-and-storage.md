# Data and storage

This page describes the storage and network behavior implemented in the
repository. It is a technical guide, not a statement about the retention settings
of the production hosting provider.

## Documents and sessions

Models and analyses are handled by the application on your device. Opening a
file reads it locally; saving on desktop writes a chosen file, and saving in the
browser creates a download. The current document open/save paths do not upload
your model to an application server.

Desktop saves stage and sync a temporary file before replacing the destination.
Browser saves report that a download was started and retain the modified marker:
the application cannot confirm that the downloaded file reached your storage.
Verify the file before closing the document. See the
[save outcomes and import limits](documents-et-compatibilite.md#import-safety-and-save-outcomes).

Session restoration stores the content of open documents, their view state and
layout locally through eframe's persistence layer. Desktop sessions use local
application storage; browser sessions use the browser's local storage for the
site. Language settings and cached document preview images are also persisted.
The session can therefore contain model text and previews even when no `.opx`
file has been saved.

There is no account-based document sync in these storage paths. Another browser,
profile, device or site origin has its own session. Private browsing, storage
limits or clearing site data can remove the browser session. Keep separate `.opx`
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
- **Saved files:** `.opx` files, exports and downloaded copies are separate.
  Remove those through your file manager if you no longer want them.

Closing a document or deleting a downloaded file does not by itself clear every
persisted session or preview, including legacy session data.

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

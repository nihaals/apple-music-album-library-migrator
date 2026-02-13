# apple-music-album-library-migrator

## Problem

If you have some songs from an album added to your library and a new version of the album releases (e.g. a deluxe), you can run into some issues:

- You can have the same song downloaded multiple times across the different album versions
- You can have the same song added to your library multiple times across the different album versions
- You can have some songs added under one version and some added under another, adding duplicate albums to your library
  - These can also have the same name and album art and only differ by track count
- If you go from the album in your library to the full album, if you added the original album version and not a later one with more songs, you might miss some newer songs

## Solution

Whenever a new version of an album is added, add the same songs in the new album version to your library and remove the previous version from your library entirely.

This is what this tool helps with. Instead of taking a screenshot of the previous version in your library and cross-referencing to see which to add in the new version, this tool will find which songs you already added, add them to the new version and remove them from the old version. You specify the source and destination album versions. It also focuses on accuracy: songs should never be incorrectly matched between the previous version and new version, even if the track number differs, so you have one less thing to worry about. You can also view the matches before doing the migration.

This tool does not make changes to any playlists and has not been tested with songs added to playlists.

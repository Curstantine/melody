# Specification

Contains the specification both the frontend and backend should follow. This documentation serves the purpose of streamlining the workflow to avoid unnecessary implementations and side-effects.

<!-- dprint-ignore-start -->
<!-- TOC -->
* [Specification](#specification)
  * [Directories](#directories)
    * [Config Directory](#config-directory)
    * [Data Directory](#data-directory)
  * [Persistence](#persistence)
    * [Primary (BonsaiDB)](#primary-bonsaidb)
    * [Settings (TOML)](#settings-toml)
  * [Models](#models)
<!-- TOC -->
<!-- dprint-ignore-end -->

## Directories

### Config Directory

The config directory contains all the files that are configurable by the user.

Paths:

- Linux: `${XDG_CONFIG_HOME}/moe.curstantine.melody/`

### Data Directory

The data directory contains all the files that are not user-editable, or are not meant to be edited by the user.

Paths:

- Linux: `${XDG_DATA_HOME}/moe.curstantine.melody/`

## Persistence

Melody uses a combination of databases and files to store its data.\
The data must stored in a way that it can be easily backed up and restored.

All these persistence methods must be implemented only in the backend, which then can be accessed by the frontend using tauri's IPC.

### Primary (BonsaiDB)

Melody uses a local BonsaiDB instance as its primary database.\
This database contains all the runtime data, such as libraries, playlists,
queues, play counts, tracks, albums and artists.

This is saved in the [`data_directory`](#data-directory) under `main.db`.

### Settings (TOML)

All the settings must be stored in a TOML file under [`config_directory/settings.toml`](#config-directory).\
These settings must be editable by the user using both the application and by opening the file in a text editor.

## Models

Different model types used to store data in the [`primary`](#primary-bonsaidb) database.

- [Library](./models/library.md)
- [Person](./models/person.md)
- [Release](./models/release.md)
- [Track](./models/track.md)
- [Resource](./models/resource.md)
